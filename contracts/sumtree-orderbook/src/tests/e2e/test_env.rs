use std::{collections::HashMap, path::PathBuf, str::FromStr};

use crate::{
    constants::{MAX_TICK, MIN_TICK},
    msg::{
        AuthExecuteMsg, CalcOutAmtGivenInResponse, DenomsResponse, ExecuteMsg,
        GetTotalPoolLiquidityResponse, GetUnrealizedCancelsResponse, InstantiateMsg,
        OrdersResponse, QueryMsg, SudoMsg, TickIdAndState, TickUnrealizedCancels, TicksResponse,
    },
    tests::test_utils::decimal256_from_u128,
    tick_math::{amount_to_value, tick_to_price, RoundingDirection},
    types::{LimitOrder, OrderDirection},
};

use cosmwasm_std::{to_json_binary, Addr, Coin, Coins, Decimal, Decimal256, Uint128, Uint256};
use osmosis_std::types::{
    cosmos::{bank::v1beta1::QueryAllBalancesRequest, base::v1beta1::Coin as ProtoCoin},
    cosmwasm::wasm::v1::MsgExecuteContractResponse,
    osmosis::cosmwasmpool::v1beta1::{
        ContractInfoByPoolIdRequest, ContractInfoByPoolIdResponse, MsgCreateCosmWasmPool,
    },
};
use osmosis_test_tube::{
    osmosis_std::types::osmosis::cosmwasmpool::v1beta1::UploadCosmWasmPoolCodeAndWhiteListProposal,
    GovWithAppAccess,
};

use osmosis_test_tube::{
    Account, Bank, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult, RunnerResult,
    SigningAccount, Wasm,
};
use serde::de::DeserializeOwned;

use super::modules::cosmwasm_pool::CosmwasmPool;

pub struct TestEnv<'a> {
    pub app: &'a OsmosisTestApp,
    pub creator: SigningAccount,
    pub contract: OrderbookContract<'a>,
    pub accounts: HashMap<String, SigningAccount>,
}

impl<'a> TestEnv<'a> {
    pub(crate) fn add_account(&mut self, username: &str, balance: Vec<Coin>) {
        let account = self.app.init_account(&balance).unwrap();
        self.accounts.insert(username.to_string(), account);
    }

    pub(crate) fn assert_contract_balances(&self, expected_balances: &[Coin], label: &str) {
        let contract_balances: Vec<Coin> = self.get_balance(&self.contract.contract_addr);

        assert_eq!(
            contract_balances, expected_balances,
            "{}: contract balances did not match",
            label
        );
    }

    pub(crate) fn get_balance(&self, address: &str) -> Vec<Coin> {
        let account_balances: Vec<Coin> = Bank::new(self.app)
            .query_all_balances(&QueryAllBalancesRequest {
                address: address.to_string(),
                pagination: None,
            })
            .unwrap()
            .balances
            .into_iter()
            .map(|coin| Coin::new(coin.amount.parse().unwrap(), coin.denom))
            .collect();

        account_balances
    }
}

pub struct TestEnvBuilder {
    account_balances: HashMap<String, Vec<Coin>>,
    instantiate_msg: Option<InstantiateMsg>,
}

impl TestEnvBuilder {
    pub(crate) fn new() -> Self {
        Self {
            account_balances: HashMap::new(),
            instantiate_msg: None,
        }
    }

    pub(crate) fn with_instantiate_msg(mut self, msg: InstantiateMsg) -> Self {
        self.instantiate_msg = Some(msg);
        self
    }

    pub(crate) fn with_account(mut self, account: &str, balance: Vec<Coin>) -> Self {
        self.account_balances.insert(account.to_string(), balance);
        self
    }
    pub(crate) fn build(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts: HashMap<_, _> = self
            .account_balances
            .into_iter()
            .map(|(account, balance)| {
                let balance: Vec<_> = balance
                    .into_iter()
                    .chain(vec![Coin::new(1000000000000, "uosmo")])
                    .collect();

                (account, app.init_account(&balance).unwrap())
            })
            .collect();

        let creator = app
            .init_account(&[Coin::new(1000000000000000u128, "uosmo")])
            .unwrap();

        let instantiate_msg = self.instantiate_msg.expect("instantiate msg not set");
        let instantiate_msg = InstantiateMsg { ..instantiate_msg };

        let contract = OrderbookContract::deploy(app, &instantiate_msg, &creator).unwrap();

        TestEnv {
            app,
            creator,
            contract,
            accounts,
        }
    }
}

pub struct OrderbookContract<'a> {
    app: &'a OsmosisTestApp,
    pub code_id: u64,
    pub pool_id: u64,
    pub contract_addr: String,
}

impl<'a> OrderbookContract<'a> {
    pub(crate) fn deploy(
        app: &'a OsmosisTestApp,
        instantiate_msg: &InstantiateMsg,
        signer: &SigningAccount,
    ) -> Result<Self, RunnerError> {
        let cp = CosmwasmPool::new(app);
        let gov = GovWithAppAccess::new(app);

        let code_id = 1; // temporary solution
        gov.propose_and_execute(
            UploadCosmWasmPoolCodeAndWhiteListProposal::TYPE_URL.to_string(),
            UploadCosmWasmPoolCodeAndWhiteListProposal {
                title: String::from("store test cosmwasm pool code"),
                description: String::from("test"),
                wasm_byte_code: Self::get_wasm_byte_code(),
            },
            signer.address(),
            signer,
        )?;

        let res = cp.create_cosmwasm_pool(
            MsgCreateCosmWasmPool {
                code_id,
                instantiate_msg: to_json_binary(instantiate_msg).unwrap().to_vec(),
                sender: signer.address(),
            },
            signer,
        )?;

        let pool_id = res.data.pool_id;

        let ContractInfoByPoolIdResponse {
            contract_address,
            code_id: _,
        } = cp.contract_info_by_pool_id(&ContractInfoByPoolIdRequest { pool_id })?;

        let contract = Self {
            app,
            code_id,
            pool_id,
            contract_addr: contract_address,
        };

        Ok(contract)
    }

    pub(crate) fn execute(
        &self,
        msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, msg, funds, signer)
    }

    pub(crate) fn query<Res>(&self, msg: &QueryMsg) -> RunnerResult<Res>
    where
        Res: ?Sized + DeserializeOwned,
    {
        let wasm = Wasm::new(self.app);
        wasm.query(&self.contract_addr, msg)
    }

    pub(crate) fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("target")
                .join("wasm32-unknown-unknown")
                .join("release")
                .join("sumtree_orderbook.wasm"),
        )
        .unwrap()
    }

    // -- Admin Methods --
    pub(crate) fn _set_admin(&self, app: &OsmosisTestApp, admin: Addr) {
        app.wasm_sudo(
            &self.contract_addr,
            SudoMsg::TransferAdmin { new_admin: admin },
        )
        .unwrap();
    }

    pub(crate) fn _set_maker_fee(
        &self,
        signer: &SigningAccount,
        maker_fee: Decimal256,
        recipient: &SigningAccount,
    ) {
        self.execute(
            &ExecuteMsg::Auth(AuthExecuteMsg::SetMakerFee { fee: maker_fee }),
            &[],
            signer,
        )
        .unwrap();

        self.execute(
            &ExecuteMsg::Auth(AuthExecuteMsg::SetMakerFeeRecipient {
                recipient: Addr::unchecked(recipient.address()),
            }),
            &[],
            signer,
        )
        .unwrap();
    }

    // -- Queries --

    pub(crate) fn get_maker_fee(&self) -> Decimal256 {
        let maker_fee: Decimal256 = self.query(&QueryMsg::GetMakerFee {}).unwrap();
        maker_fee
    }

    pub(crate) fn get_denoms(&self) -> DenomsResponse {
        self.query(&QueryMsg::Denoms {}).unwrap()
    }

    // Calculate the expected output for a given input amount/direction using the CosmWasm pool query
    pub(crate) fn get_out_given_in(
        &self,
        direction: OrderDirection,
        amount: impl Into<u128>,
    ) -> RunnerResult<ProtoCoin> {
        let (token_in_denom, token_out_denom) = if direction == OrderDirection::Bid {
            (self.get_denoms().quote_denom, self.get_denoms().base_denom)
        } else {
            (self.get_denoms().base_denom, self.get_denoms().quote_denom)
        };

        self.query(&QueryMsg::CalcOutAmountGivenIn {
            token_in: Coin::new(amount.into(), token_in_denom),
            token_out_denom,
            swap_fee: Decimal::zero(),
        })
        .map(|r: CalcOutAmtGivenInResponse| r.token_out)
    }

    pub(crate) fn get_directional_liquidity(&self, order_direction: OrderDirection) -> u128 {
        let GetTotalPoolLiquidityResponse {
            total_pool_liquidity,
        } = self.query(&QueryMsg::GetTotalPoolLiquidity {}).unwrap();

        // Determine the amount of liquidity for the given direction
        let liquidity = if order_direction == OrderDirection::Bid {
            Coins::try_from(total_pool_liquidity.clone())
                .unwrap()
                .amount_of("base")
        } else {
            Coins::try_from(total_pool_liquidity.clone())
                .unwrap()
                .amount_of("quote")
        };

        liquidity.u128()
    }

    pub(crate) fn get_order(
        &self,
        sender: String,
        tick_id: i64,
        order_id: u64,
    ) -> Option<LimitOrder> {
        let OrdersResponse { orders, .. } = self
            .query(&QueryMsg::OrdersByOwner {
                owner: Addr::unchecked(sender),
                start_from: Some((tick_id, order_id)),
                end_at: None,
                limit: Some(1),
            })
            .unwrap();
        orders.first().cloned()
    }

    pub(crate) fn collect_all_ticks(&self) -> Vec<TickIdAndState> {
        let mut ticks = vec![];
        let mut min_tick = MIN_TICK;
        while min_tick <= MAX_TICK {
            let tick: TicksResponse = self
                .query(&QueryMsg::AllTicks {
                    start_from: Some(min_tick),
                    end_at: Some(MAX_TICK),
                    limit: Some(300),
                })
                .unwrap();
            if tick.ticks.is_empty() {
                break;
            }
            ticks.extend(tick.ticks.clone());
            // Determine the next tick to start at for the next query loop
            min_tick = tick.ticks.iter().max_by_key(|t| t.tick_id).unwrap().tick_id + 1;
        }
        ticks
    }

    pub(crate) fn collect_all_orders(&self) -> Vec<LimitOrder> {
        let ticks = self.collect_all_ticks();

        let mut all_orders: Vec<LimitOrder> = vec![];
        for tick in ticks {
            let orders: OrdersResponse = self
                .query(&QueryMsg::OrdersByTick {
                    tick_id: tick.tick_id,
                    start_from: None,
                    end_at: None,
                    limit: None,
                })
                .unwrap();
            all_orders.extend(orders.orders.clone());
        }

        all_orders
    }

    /// Calculates the max amount for a market order that can be placed
    /// by iterating over all the ticks, calculating their liquidity and summing
    ///
    /// The amount caps at `u128::MAX`
    pub(crate) fn get_max_market_amount(&self, direction: OrderDirection) -> u128 {
        let mut max_amount: Uint128 = Uint128::zero();
        let ticks = self.collect_all_ticks();
        for tick in ticks {
            let value = tick.tick_state.get_values(direction.opposite());

            // If the tick has no liquidity we can skip this tick
            if value.total_amount_of_liquidity.is_zero() {
                continue;
            }

            let price = tick_to_price(tick.tick_id).unwrap();
            let amount_of_liquidity = Uint128::from_str(
                &(value
                    .total_amount_of_liquidity
                    .min(decimal256_from_u128(u128::MAX)))
                .to_string(),
            )
            .unwrap();
            let amount_u256 = amount_to_value(
                direction.opposite(),
                amount_of_liquidity,
                price,
                RoundingDirection::Down,
            )
            .unwrap();
            let amount =
                Uint128::from_str(&(amount_u256.min(Uint256::from_u128(u128::MAX))).to_string())
                    .unwrap();

            max_amount = max_amount.saturating_add(amount);
        }
        max_amount.u128()
    }

    /// Calculates how much is available for claim for a given order
    ///
    /// The amount that is claimable is dependent on a tick sync.
    /// To account for this we first fetch the amount of unrealized cancels for the tick
    /// and add it to the current ETAS before computing the difference.
    pub(crate) fn get_order_claimable_amount(&self, order: LimitOrder) -> u128 {
        let TicksResponse { ticks } = self
            .query(&QueryMsg::AllTicks {
                start_from: Some(order.tick_id),
                end_at: None,
                limit: Some(1),
            })
            .unwrap();

        // Get current tick values
        let tick = ticks.first().unwrap().tick_state.clone();
        let tick_values = tick.get_values(order.order_direction);

        // Get the current unrealized cancels for the tick
        let GetUnrealizedCancelsResponse { ticks } = self
            .query(&QueryMsg::GetUnrealizedCancels {
                tick_ids: vec![order.tick_id],
            })
            .unwrap();
        let TickUnrealizedCancels {
            unrealized_cancels, ..
        } = ticks.first().unwrap();
        let cancelled_amount = match order.order_direction {
            OrderDirection::Bid => unrealized_cancels.bid_unrealized_cancels,
            OrderDirection::Ask => unrealized_cancels.ask_unrealized_cancels,
        };

        // Add unrealized cancels to the current ETAS
        let synced_etas = tick_values
            .effective_total_amount_swapped
            .checked_add(cancelled_amount)
            .unwrap();

        // Compute the expected amount as if the tick had been synced
        let expected_amount_u256 = synced_etas
            .saturating_sub(order.etas)
            .to_uint_floor()
            .min(Uint256::from(order.quantity.u128()));

        let expected_amount = Uint128::try_from(expected_amount_u256).unwrap();
        expected_amount.u128()
    }
}

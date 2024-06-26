use std::{collections::HashMap, path::PathBuf};

use crate::{
    constants::{MAX_TICK, MIN_TICK},
    msg::{
        AuthExecuteMsg, AuthQueryMsg, DenomsResponse, ExecuteMsg, GetTotalPoolLiquidityResponse,
        GetUnrealizedCancelsResponse, InstantiateMsg, OrdersResponse, QueryMsg, SudoMsg,
        TickIdAndState, TickUnrealizedCancels, TicksResponse,
    },
    types::{LimitOrder, OrderDirection},
    ContractError,
};

use cosmwasm_std::{to_json_binary, Addr, Coin, Coins, Decimal256, Uint128, Uint256};
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryAllBalancesRequest,
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
    pub fn add_account(&mut self, username: &str, balance: Vec<Coin>) {
        let account = self.app.init_account(&balance).unwrap();
        self.accounts.insert(username.to_string(), account);
    }

    pub fn _assert_account_balances(
        &self,
        account: &str,
        expected_balances: Vec<Coin>,
        ignore_denoms: Vec<&str>,
    ) {
        let account_balances: Vec<Coin> = self
            ._get_account_balance(account)
            .iter()
            .filter(|coin| !ignore_denoms.contains(&coin.denom.as_str()))
            .cloned()
            .collect();

        assert_eq!(account_balances, expected_balances);
    }

    pub fn assert_contract_balances(&self, expected_balances: &[Coin], label: &str) {
        let contract_balances: Vec<Coin> = self.get_balance(&self.contract.contract_addr);

        assert_eq!(
            contract_balances, expected_balances,
            "{}: contract balances did not match",
            label
        );
    }

    pub fn get_balance(&self, address: &str) -> Vec<Coin> {
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

    pub fn _get_account_balance(&self, account: &str) -> Vec<Coin> {
        let account = self.accounts.get(account).unwrap();

        self.get_balance(&account.address())
    }
}

pub struct TestEnvBuilder {
    account_balances: HashMap<String, Vec<Coin>>,
    instantiate_msg: Option<InstantiateMsg>,
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        Self {
            account_balances: HashMap::new(),
            instantiate_msg: None,
        }
    }

    pub fn with_instantiate_msg(mut self, msg: InstantiateMsg) -> Self {
        self.instantiate_msg = Some(msg);
        self
    }

    pub fn with_account(mut self, account: &str, balance: Vec<Coin>) -> Self {
        self.account_balances.insert(account.to_string(), balance);
        self
    }
    pub fn build(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
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
    pub fn deploy(
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

    pub fn execute(
        &self,
        msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, msg, funds, signer)
    }

    pub fn query<Res>(&self, msg: &QueryMsg) -> RunnerResult<Res>
    where
        Res: ?Sized + DeserializeOwned,
    {
        let wasm = Wasm::new(self.app);
        wasm.query(&self.contract_addr, msg)
    }

    pub fn get_wasm_byte_code() -> Vec<u8> {
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

    pub fn _set_admin(&self, app: &OsmosisTestApp, admin: Addr) {
        app.wasm_sudo(
            &self.contract_addr,
            SudoMsg::TransferAdmin { new_admin: admin },
        )
        .unwrap();
        let admin: Option<Addr> = self.query(&QueryMsg::Auth(AuthQueryMsg::Admin {})).unwrap();
        println!("admin_set: {:?}", admin);
    }

    pub fn _set_maker_fee(
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

    pub fn get_order_claimable_amount(&self, order: LimitOrder) -> u128 {
        let TicksResponse { ticks } = self
            .query(&QueryMsg::AllTicks {
                start_from: Some(order.tick_id),
                end_at: None,
                limit: Some(1),
            })
            .unwrap();
        let tick = ticks.first().unwrap().tick_state.clone();
        let tick_values: crate::types::TickValues = tick.get_values(order.order_direction);
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

        let synced_etas = tick_values
            .effective_total_amount_swapped
            .checked_add(cancelled_amount)
            .unwrap();
        let expected_amount_u256 = synced_etas
            .saturating_sub(order.etas)
            .to_uint_floor()
            .min(Uint256::from(order.quantity.u128()));

        let expected_amount = Uint128::try_from(expected_amount_u256).unwrap();
        expected_amount.u128()
    }

    pub fn get_maker_fee(&self) -> Decimal256 {
        let maker_fee: Decimal256 = self.query(&QueryMsg::GetMakerFee {}).unwrap();
        maker_fee
    }

    pub fn get_denoms(&self) -> DenomsResponse {
        self.query(&QueryMsg::Denoms {}).unwrap()
    }

    pub fn collect_all_ticks(&self) -> Vec<TickIdAndState> {
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
            min_tick = tick.ticks.iter().max_by_key(|t| t.tick_id).unwrap().tick_id + 1;
        }
        ticks
    }

    pub fn get_directional_liquidity(&self, order_direction: OrderDirection) -> u128 {
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

    pub fn get_order(&self, sender: String, tick_id: i64, order_id: u64) -> Option<LimitOrder> {
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
}

pub fn _assert_contract_err(expected: ContractError, actual: RunnerError) {
    match actual {
        RunnerError::ExecuteError { msg } => {
            if !msg.contains(&expected.to_string()) {
                panic!(
                    "assertion failed:\n\n  must contain \t: \"{}\",\n  actual \t: \"{}\"\n",
                    expected, msg
                )
            }
        }
        _ => panic!("unexpected error, expect execute error but got: {}", actual),
    };
}

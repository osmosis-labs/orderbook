/// Sets up the testing environment for the orderbook
#[macro_export]
macro_rules! setup {
    ($($app:expr, $quote_denom:expr, $base_denom:expr, $maker_fee:expr),* ) => {{
        $($app.init_account(&[
            cosmwasm_std::Coin::new(1, $quote_denom),
            cosmwasm_std::Coin::new(1, $base_denom),
        ])
        .unwrap();

        // use osmosis_test_tube::Account;
        let t = $crate::tests::e2e::test_env::TestEnvBuilder::new()
            .with_account(
                "user1",
                vec![
                    cosmwasm_std::Coin::new(2_000, $quote_denom),
                    cosmwasm_std::Coin::new(2_000, $base_denom),
                ],
            )
            .with_account(
                "user2",
                vec![
                    cosmwasm_std::Coin::new(2_000, $quote_denom),
                    cosmwasm_std::Coin::new(2_000, $base_denom),
                ],
            )
            .with_account(
                "contract_admin",
                vec![],
            )
            .with_account(
                "maker_fee_recipient",
                vec![],
            )
            .with_instantiate_msg($crate::msg::InstantiateMsg {
                base_denom: $base_denom.to_string(),
                quote_denom: $quote_denom.to_string(),
            })
            .build(&$app);

        let $crate::msg::DenomsResponse {
            quote_denom,
            base_denom,
        } = t.contract.query(&$crate::msg::QueryMsg::Denoms {}).unwrap();

        assert_eq!(quote_denom, "quote");
        assert_eq!(base_denom, "base");

        let $crate::msg::GetTotalPoolLiquidityResponse {
            total_pool_liquidity,
        } = t
            .contract
            .query(&$crate::msg::QueryMsg::GetTotalPoolLiquidity {})
            .unwrap();

        assert_eq!(
            total_pool_liquidity,
            vec![
                cosmwasm_std::Coin::new(0, "base"),
                cosmwasm_std::Coin::new(0, "quote"),
            ]
        );

        let is_active: bool = t.contract.query(&$crate::msg::QueryMsg::IsActive {}).unwrap();

        assert!(is_active);

        // NOTE: wasm_sudo does not currently maintain state so these calls will not work
        // t.contract.set_admin($app, cosmwasm_std::Addr::unchecked(&t.accounts["contract_admin"].address()));
        // t.contract
        //     .set_maker_fee(&t.accounts["contract_admin"], Decimal256::percent($maker_fee), &t.accounts["maker_fee_recipient"]);

        t)*
    }};
}

// -- Assertions --
// Assertions about current state
pub mod assert {
    use crate::{
        msg::{
            DenomsResponse, GetTotalPoolLiquidityResponse, GetUnrealizedCancelsResponse, QueryMsg,
            SpotPriceResponse,
        },
        tests::e2e::test_env::TestEnv,
        tick_math::tick_to_price,
        types::{OrderDirection, Orderbook},
    };
    use cosmwasm_std::{Coin, Coins};
    use osmosis_test_tube::{cosmrs::proto::prost::Message, RunnerExecuteResult};

    // -- Contract State Assertions

    /// Asserts that the orderbook's current liquidity matches what is provided
    pub fn pool_liquidity(
        t: &TestEnv,
        base_liquidity: impl Into<u128>,
        quote_liquidity: impl Into<u128>,
        label: &str,
    ) {
        let DenomsResponse {
            quote_denom,
            base_denom,
        } = t.contract.get_denoms();
        let GetTotalPoolLiquidityResponse {
            total_pool_liquidity,
        } = t
            .contract
            .query(&QueryMsg::GetTotalPoolLiquidity {})
            .unwrap();
        assert_eq!(
            total_pool_liquidity,
            vec![
                Coin::new(base_liquidity.into(), base_denom),
                Coin::new(quote_liquidity.into(), quote_denom)
            ],
            "{}: pool liquidity did not match",
            label
        );
    }

    /// Asserts that the contract's balance matches what is provided
    pub fn pool_balance(
        t: &TestEnv,
        base_liquidity: impl Into<u128>,
        quote_liquidity: impl Into<u128>,
        label: &str,
    ) {
        let DenomsResponse {
            quote_denom,
            base_denom,
        } = t.contract.get_denoms();
        t.assert_contract_balances(
            [
                Coin::new(base_liquidity.into(), base_denom),
                Coin::new(quote_liquidity.into(), quote_denom),
            ]
            .iter()
            .filter(|x| !x.amount.is_zero())
            .cloned()
            .collect::<Vec<Coin>>()
            .as_slice(),
            label,
        );
    }

    /// Asserts that the orderbook spot price matches what is provided
    pub fn spot_price(t: &TestEnv, bid_tick: i64, ask_tick: i64, label: &str) {
        let bid_price = tick_to_price(bid_tick).unwrap();
        let ask_price = tick_to_price(ask_tick).unwrap();
        let DenomsResponse {
            quote_denom,
            base_denom,
        } = t.contract.get_denoms();

        for (base_denom, quote_denom, price, direction) in [
            (base_denom.clone(), quote_denom.clone(), ask_price, "ask"),
            (quote_denom, base_denom, bid_price, "bid"),
        ] {
            let SpotPriceResponse { spot_price } = t
                .contract
                .query(&QueryMsg::SpotPrice {
                    base_asset_denom: base_denom,
                    quote_asset_denom: quote_denom,
                })
                .unwrap();

            assert_eq!(
                spot_price.to_string(),
                price.to_string(),
                "{}: {} price did not match",
                label,
                direction
            );
        }
    }

    /// Asserts that the contract balance is greater than or equal to what is recorded in the orderbook directional liquidity state
    /// If this assertion is ever false then the orderbook is "out of balance" and cannot provide liquidity for future orders
    pub fn has_liquidity(t: &TestEnv) {
        let bid_liquidity = t.contract.get_directional_liquidity(OrderDirection::Bid);
        let ask_liquidity = t.contract.get_directional_liquidity(OrderDirection::Ask);

        let balance = Coins::try_from(t.get_balance(&t.contract.contract_addr)).unwrap();
        let bid_balance = balance.amount_of(&t.contract.get_denoms().base_denom);
        let ask_balance = balance.amount_of(&t.contract.get_denoms().quote_denom);

        assert!(
            bid_liquidity <= bid_balance.u128(),
            "invalid bid liquidity, expected: {}, got: {}",
            bid_balance,
            bid_liquidity
        );
        assert!(
            ask_liquidity <= ask_balance.u128(),
            "invalid ask liquidity, expected: {}, got: {}",
            ask_balance,
            ask_liquidity
        );
    }

    /// Assertions about tick state
    /// 1. All ticks have a cumulative value that is greater than or equal to the effective total amount swapped
    /// 2. The next ask tick is less than or equal to the minimum tick with an ask amount
    /// 3. The next bid tick is greater than or equal to the maximum tick with a bid amount
    ///
    /// This assertion can be run mid test as it must always be true
    pub fn tick_invariants(t: &TestEnv) {
        let ticks = t.contract.collect_all_ticks();

        assert!(ticks
            .iter()
            .all(|t| t.tick_state.ask_values.effective_total_amount_swapped
                <= t.tick_state.ask_values.cumulative_total_value));
        assert!(ticks
            .iter()
            .all(|t| t.tick_state.bid_values.effective_total_amount_swapped
                <= t.tick_state.bid_values.cumulative_total_value));

        let ticks_with_bid_amount = ticks.iter().filter(|tick| {
            !tick
                .tick_state
                .get_values(OrderDirection::Bid)
                .total_amount_of_liquidity
                .is_zero()
        });
        let ticks_with_ask_amount = ticks.iter().filter(|tick| {
            !tick
                .tick_state
                .get_values(OrderDirection::Ask)
                .total_amount_of_liquidity
                .is_zero()
        });
        let max_tick_with_bid = ticks_with_bid_amount.max_by_key(|tick| tick.tick_id);
        let min_tick_with_ask = ticks_with_ask_amount.min_by_key(|tick| tick.tick_id);

        let Orderbook {
            next_ask_tick,
            next_bid_tick,
            ..
        } = t.contract.query(&QueryMsg::OrderbookState {}).unwrap();
        if let Some(min_tick_with_ask) = min_tick_with_ask {
            assert!(next_ask_tick <= min_tick_with_ask.tick_id);
        }
        if let Some(max_tick_with_bid) = max_tick_with_bid {
            assert!(next_bid_tick >= max_tick_with_bid.tick_id);
        }
    }

    /// Asserts that there are no remaining orders in the orderbook
    pub fn no_remaining_orders(t: &TestEnv) {
        let all_orders = t.contract.collect_all_orders();
        assert_eq!(all_orders.len(), 0);
    }

    /// Asserts that all ticks are fully synced
    ///
    /// **Should be run AFTER a fuzz test**
    pub fn clean_ticks(t: &TestEnv) {
        let all_ticks = t.contract.collect_all_ticks();
        for tick in all_ticks {
            let GetUnrealizedCancelsResponse { ticks } = t
                .contract
                .query(&QueryMsg::GetUnrealizedCancels {
                    tick_ids: vec![tick.tick_id],
                })
                .unwrap();
            let unrealized_cancels_state = ticks.first().unwrap();
            for direction in [OrderDirection::Ask, OrderDirection::Bid] {
                let values = tick.tick_state.get_values(direction);
                assert!(
                    values.total_amount_of_liquidity.is_zero(),
                    "tick {} has liquidity",
                    tick.tick_id
                );

                let unrealized_cancels = match direction {
                    OrderDirection::Ask => {
                        unrealized_cancels_state
                            .unrealized_cancels
                            .ask_unrealized_cancels
                    }
                    OrderDirection::Bid => {
                        unrealized_cancels_state
                            .unrealized_cancels
                            .bid_unrealized_cancels
                    }
                };

                // As a tick may not be fully synced due to the last order being a cancellation rather than a claim
                // we check that if the tick was fully synced then ETAS == CTT must be true
                // In the case that the tick was already synced then unrealized cancels is 0 and we are doing a direct
                // ETAS == CTT comparison
                assert_eq!(
                    values
                        .effective_total_amount_swapped
                        .checked_add(unrealized_cancels)
                        .unwrap(),
                    values.cumulative_total_value
                );
            }
        }
    }

    // -- Balance Assertions --

    /// An assertion that records balances before an action and compares the balances after the provided action
    /// Comparisons are only done for the vector of addresses provided in the second parameter
    pub fn balance_changes<T: Message + Default>(
        t: &TestEnv,
        changes: &[(&str, Vec<Coin>)],
        action: impl FnOnce() -> RunnerExecuteResult<T>,
    ) -> RunnerExecuteResult<T> {
        // Record balances before the action
        let pre_balances: Vec<(String, Coins)> = changes
            .iter()
            .map(|(sender, _)| {
                (
                    sender.to_string(),
                    Coins::try_from(t.get_balance(sender)).unwrap(),
                )
            })
            .collect();

        // Run the action
        let result = action()?;

        // Check balances after running the action
        let post_balances: Vec<(String, Coins)> = changes
            .iter()
            .map(|(sender, _)| {
                (
                    sender.to_string(),
                    Coins::try_from(t.get_balance(sender)).unwrap(),
                )
            })
            .collect();

        // Check all expected balance changes
        for (sender, balance_change) in changes.iter().cloned() {
            let pre_balance = pre_balances
                .iter()
                .find(|(s, _)| s == sender)
                .unwrap()
                .1
                .clone();
            let post_balance = post_balances
                .iter()
                .find(|(s, _)| s == sender)
                .unwrap()
                .1
                .clone();

            for coin in balance_change {
                let pre_amount = pre_balance.amount_of(&coin.denom);
                let post_amount = post_balance.amount_of(&coin.denom);
                let change = post_amount.saturating_sub(pre_amount);
                assert_eq!(
                    change, coin.amount,
                    "Did not receive expected amount change, expected: {}{}, got: {}{}",
                    coin.amount, coin.denom, change, coin.denom
                );
            }
        }

        Ok(result)
    }
}

/// Utili functions for interacting with the orderbook
pub mod orders {
    use std::str::FromStr;

    use cosmwasm_std::{coins, Coin, Decimal, Decimal256, Uint128, Uint256};

    use osmosis_std::types::{
        cosmwasm::wasm::v1::MsgExecuteContractResponse,
        osmosis::poolmanager::v1beta1::{
            MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
        },
    };
    use osmosis_test_tube::{Account, OsmosisTestApp, RunnerExecuteResult};

    use crate::{
        msg::{CalcOutAmtGivenInResponse, DenomsResponse, ExecuteMsg, QueryMsg},
        tests::e2e::{modules::cosmwasm_pool::CosmwasmPool, test_env::TestEnv},
        tick_math::{amount_to_value, tick_to_price, RoundingDirection},
        types::{LimitOrder, OrderDirection},
    };

    use super::assert;

    pub fn place_limit(
        t: &TestEnv,
        tick_id: i64,
        order_direction: OrderDirection,
        quantity: impl Into<Uint128>,
        claim_bounty: Option<Decimal256>,
        sender: &str,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let DenomsResponse {
            quote_denom,
            base_denom,
        } = t.contract.query(&QueryMsg::Denoms {}).unwrap();

        let denom = if order_direction == OrderDirection::Bid {
            quote_denom
        } else {
            base_denom
        };

        let quantity_u128: Uint128 = quantity.into();

        t.contract.execute(
            &ExecuteMsg::PlaceLimit {
                tick_id,
                order_direction,
                quantity: quantity_u128,
                claim_bounty,
            },
            &coins(quantity_u128.u128(), denom),
            &t.accounts[sender],
        )
    }

    pub fn place_market(
        cp: &CosmwasmPool<OsmosisTestApp>,
        t: &TestEnv,
        order_direction: OrderDirection,
        quantity: impl Into<Uint128>,
        sender: &str,
    ) -> RunnerExecuteResult<MsgSwapExactAmountInResponse> {
        let pool_id = t.contract.pool_id;
        let quantity_u128: Uint128 = quantity.into();
        let DenomsResponse {
            base_denom,
            quote_denom,
        } = t.contract.query(&QueryMsg::Denoms {}).unwrap();

        let token_out_denom = if order_direction == OrderDirection::Bid {
            base_denom.clone()
        } else {
            quote_denom.clone()
        };
        let token_in_denom = if order_direction == OrderDirection::Bid {
            quote_denom
        } else {
            base_denom
        };

        cp.swap_exact_amount_in(
            MsgSwapExactAmountIn {
                sender: t.accounts[sender].address(),
                routes: vec![SwapAmountInRoute {
                    pool_id,
                    token_out_denom,
                }],
                token_in: Some(Coin::new(quantity_u128.u128(), token_in_denom).into()),
                token_out_min_amount: Uint128::one().to_string(),
            },
            &t.accounts[sender],
        )
    }

    /// Places a market order and asserts that the sender's balance changes correctly
    ///
    /// Note: this check has some circularity to it as the expected out depends on the `CalcOutAmtGivenInResponse`
    pub fn place_market_and_assert_balance(
        cp: &CosmwasmPool<OsmosisTestApp>,
        t: &TestEnv,
        order_direction: OrderDirection,
        quantity: impl Into<Uint128> + Clone,
        sender: &str,
    ) -> RunnerExecuteResult<MsgSwapExactAmountInResponse> {
        let quantity_u128: Uint128 = quantity.clone().into();
        let DenomsResponse {
            base_denom,
            quote_denom,
        } = t.contract.query(&QueryMsg::Denoms {}).unwrap();

        let token_out_denom = if order_direction == OrderDirection::Bid {
            base_denom.clone()
        } else {
            quote_denom.clone()
        };
        let token_in_denom = if order_direction == OrderDirection::Bid {
            quote_denom
        } else {
            base_denom
        };

        let CalcOutAmtGivenInResponse { token_out } = t
            .contract
            .query(&QueryMsg::CalcOutAmountGivenIn {
                token_in: Coin::new(quantity_u128.u128(), token_in_denom.clone()),
                token_out_denom,
                swap_fee: Decimal::zero(),
            })
            .unwrap();

        assert::balance_changes(
            t,
            // Users receives expected amount out in token out denom
            &[(
                &t.accounts[sender].address(),
                vec![Coin::new(
                    Uint128::from_str(&token_out.amount.to_string())
                        .unwrap()
                        .u128(),
                    token_out.denom,
                )],
            )],
            || place_market(cp, t, order_direction, quantity, sender),
        )
    }

    pub fn claim(
        t: &TestEnv,
        sender: &str,
        tick_id: i64,
        order_id: u64,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        t.contract.execute(
            &ExecuteMsg::ClaimLimit { order_id, tick_id },
            &[],
            &t.accounts[sender],
        )
    }

    /// Claims a given order using the provided sender account name
    ///
    /// Asserts that the sender and order owner's balances change correctly
    pub fn claim_and_assert_balance(
        t: &TestEnv,
        sender: &str,
        owner: &str,
        tick_id: i64,
        order_id: u64,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let order: LimitOrder = t
            .contract
            .get_order(t.accounts[owner].address(), tick_id, order_id)
            .unwrap();

        // Get how much is expected out given the current tick state (accounts for unrealized cancels)
        let expected_amount = t.contract.get_order_claimable_amount(order.clone());

        // Convert the expected amount to the price of the order
        let price = tick_to_price(order.tick_id).unwrap();
        let mut expected_received_u256 = amount_to_value(
            order.order_direction,
            Uint128::from(expected_amount),
            price,
            RoundingDirection::Down,
        )
        .unwrap();
        // Create immutable expected received for calculating claim and maker fees
        let immut_expected_received_u256 = expected_received_u256;

        // Calculate the bounty amount if there is one
        let mut bounty_amount_256 = Uint256::zero();
        if let Some(bounty) = order.claim_bounty {
            if order.owner != t.accounts[sender].address() {
                bounty_amount_256 =
                    Decimal256::from_ratio(immut_expected_received_u256, Uint256::one())
                        .checked_mul(bounty)
                        .unwrap()
                        .to_uint_floor();
                // Subtract the bounty from the expected received
                expected_received_u256 = expected_received_u256
                    .checked_sub(bounty_amount_256)
                    .unwrap();
            }
        }

        // Calculate the maker fee
        // May be zero
        let maker_fee = t.contract.get_maker_fee();
        let maker_fee_amount_u256 =
            Decimal256::from_ratio(immut_expected_received_u256, Uint256::one())
                .checked_mul(maker_fee)
                .unwrap()
                .to_uint_floor();
        let maker_fee_amount = Uint128::try_from(maker_fee_amount_u256).unwrap();

        // Subtract the maker fee from the expected received
        expected_received_u256 = expected_received_u256
            .checked_sub(maker_fee_amount_u256)
            .unwrap();

        let bounty_amount = Uint128::try_from(bounty_amount_256).unwrap();
        let expected_received = Uint128::try_from(expected_received_u256).unwrap();

        let DenomsResponse {
            base_denom,
            quote_denom,
        } = t.contract.get_denoms();
        let expected_denom = if order.order_direction == OrderDirection::Bid {
            base_denom
        } else {
            quote_denom
        };

        assert::balance_changes(
            t,
            [
                // Assert owner receives amount - maker fee - claim bounty
                (
                    order.owner.as_str(),
                    vec![Coin::new(expected_received.u128(), expected_denom.clone())],
                ),
                // Assert sender receives bounty (will be 0 if the sender is the owner)
                (
                    &t.accounts[sender].address(),
                    vec![Coin::new(bounty_amount.u128(), expected_denom.clone())],
                ),
                // Assert maker fee recipient receives maker fee
                (
                    &t.accounts["maker_fee_recipient"].address(),
                    vec![Coin::new(maker_fee_amount.u128(), expected_denom)],
                ),
            ]
            .iter()
            // Remove any 0 checks
            .filter(|x| x.1.iter().all(|y| !y.amount.is_zero()))
            .cloned()
            .collect::<Vec<(&str, Vec<Coin>)>>()
            .as_slice(),
            || claim(t, sender, tick_id, order_id),
        )
    }

    pub fn cancel_limit(
        t: &TestEnv,
        sender: &str,
        tick_id: i64,
        order_id: u64,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        t.contract.execute(
            &ExecuteMsg::CancelLimit { order_id, tick_id },
            &[],
            &t.accounts[sender],
        )
    }

    /// Cancels a limit order and asserts that the owner receives back the remaining order quantity (may be partially filled)
    pub fn cancel_limit_and_assert_balance(
        t: &TestEnv,
        sender: &str,
        tick_id: i64,
        order_id: u64,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let order = t
            .contract
            .get_order(t.accounts[sender].address(), tick_id, order_id)
            .unwrap();

        let order_direction = order.order_direction;
        let quantity = order.quantity;
        let DenomsResponse {
            base_denom,
            quote_denom,
        } = t.contract.get_denoms();
        let token_in_denom = if order_direction == OrderDirection::Bid {
            quote_denom
        } else {
            base_denom
        };

        assert::balance_changes(
            t,
            // Assert owner receives back the remaining order quantity
            &[(
                &t.accounts[sender].address(),
                vec![Coin::new(quantity.u128(), token_in_denom)],
            )],
            || cancel_limit(t, sender, tick_id, order_id),
        )
    }
}

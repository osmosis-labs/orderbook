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

        // t.contract.set_admin($app, cosmwasm_std::Addr::unchecked(&t.accounts["contract_admin"].address()));
        // t.contract
        //     .set_maker_fee(&t.accounts["contract_admin"], Decimal256::percent($maker_fee), &t.accounts["maker_fee_recipient"]);

        t)*
    }};
}

pub mod assert {
    use crate::{
        msg::{DenomsResponse, GetTotalPoolLiquidityResponse, QueryMsg, SpotPriceResponse},
        tests::e2e::test_env::TestEnv,
        tick_math::tick_to_price,
        types::{OrderDirection, Orderbook},
    };
    use cosmwasm_std::{Coin, Coins};
    use osmosis_test_tube::{cosmrs::proto::prost::Message, RunnerExecuteResult};

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

    pub fn balance_changes<T: Message + Default>(
        t: &TestEnv,
        changes: &[(&str, Vec<Coin>)],
        action: impl FnOnce() -> RunnerExecuteResult<T>,
    ) -> RunnerExecuteResult<T> {
        let pre_balances: Vec<(String, Coins)> = changes
            .iter()
            .map(|(sender, _)| {
                (
                    sender.to_string(),
                    Coins::try_from(t.get_balance(sender)).unwrap(),
                )
            })
            .collect();
        let result = action();
        let post_balances: Vec<(String, Coins)> = changes
            .iter()
            .map(|(sender, _)| {
                (
                    sender.to_string(),
                    Coins::try_from(t.get_balance(sender)).unwrap(),
                )
            })
            .collect();

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

        result
    }

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
}

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
        msg::{
            CalcOutAmtGivenInResponse, DenomsResponse, ExecuteMsg, QueryMsg, TickUnrealizedCancels,
            TickUnrealizedCancelsByIdResponse, TicksResponse,
        },
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

    pub fn place_market_success(
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

    pub fn claim_success(
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
        let TicksResponse { ticks } = t
            .contract
            .query(&QueryMsg::AllTicks {
                start_from: Some(order.tick_id),
                end_at: None,
                limit: Some(1),
            })
            .unwrap();
        let tick = ticks.first().unwrap().tick_state.clone();
        let tick_values: crate::types::TickValues = tick.get_values(order.order_direction);
        let TickUnrealizedCancelsByIdResponse { ticks } = t
            .contract
            .query(&QueryMsg::TickUnrealizedCancelsById {
                tick_ids: vec![tick_id],
            })
            .unwrap();
        let TickUnrealizedCancels {
            unrealized_cancels, ..
        } = ticks.first().unwrap();
        let cancelled_amount = match order.order_direction {
            OrderDirection::Bid => unrealized_cancels.bid_unrealized_cancels,
            OrderDirection::Ask => unrealized_cancels.ask_unrealized_cancels,
        }
        .checked_sub(tick_values.cumulative_realized_cancels)
        .unwrap();

        let expected_amount_u256 = tick_values
            .effective_total_amount_swapped
            .checked_add(cancelled_amount)
            .unwrap()
            .checked_sub(order.etas)
            .unwrap()
            .to_uint_floor()
            .min(Uint256::from(order.quantity.u128()));

        let expected_amount = Uint128::try_from(expected_amount_u256).unwrap();
        let price = tick_to_price(order.tick_id).unwrap();
        let mut expected_received_u256 = amount_to_value(
            order.order_direction,
            expected_amount,
            price,
            RoundingDirection::Down,
        )
        .unwrap();
        let immut_expected_received_u256 = expected_received_u256;

        let mut bounty_amount_256 = Uint256::zero();
        if let Some(bounty) = order.claim_bounty {
            if order.owner != t.accounts[sender].address() {
                bounty_amount_256 =
                    Decimal256::from_ratio(immut_expected_received_u256, Uint256::one())
                        .checked_mul(bounty)
                        .unwrap()
                        .to_uint_floor();
                expected_received_u256 = expected_received_u256
                    .checked_sub(bounty_amount_256)
                    .unwrap();
            }
        }

        let maker_fee = t.contract.get_maker_fee();
        let maker_fee_amount_u256 =
            Decimal256::from_ratio(immut_expected_received_u256, Uint256::one())
                .checked_mul(maker_fee)
                .unwrap()
                .to_uint_floor();
        let maker_fee_amount = Uint128::try_from(maker_fee_amount_u256).unwrap();

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
                (
                    order.owner.as_str(),
                    vec![Coin::new(expected_received.u128(), expected_denom.clone())],
                ),
                (
                    &t.accounts[sender].address(),
                    vec![Coin::new(bounty_amount.u128(), expected_denom.clone())],
                ),
                (
                    &t.accounts["maker_fee_recipient"].address(),
                    vec![Coin::new(maker_fee_amount.u128(), expected_denom)],
                ),
            ]
            .iter()
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

    pub fn cancel_limit_success(t: &TestEnv, sender: &str, tick_id: i64, order_id: u64) {
        let order: LimitOrder = t
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
            &[(
                &t.accounts[sender].address(),
                vec![Coin::new(quantity.u128(), token_in_denom)],
            )],
            || cancel_limit(t, sender, tick_id, order_id),
        )
        .unwrap();
    }
}

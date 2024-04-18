use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Decimal, Decimal256, Uint128,
};

use crate::{
    contract::query_spot_price,
    orderbook::create_orderbook,
    types::{LimitOrder, OrderDirection},
    ContractError,
};

use super::test_utils::{format_test_name, OrderOperation};

struct SpotPriceTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    base_denom: String,
    quote_denom: String,
    expected_price: Decimal,
    expected_error: Option<ContractError>,
}

#[test]
fn test_query_spot_price() {
    let sender = Addr::unchecked("sender");
    let base_denom = "base";
    let quote_denom = "quote";
    let test_cases: Vec<SpotPriceTestCase> = vec![SpotPriceTestCase {
        name: "ASK: basic price 1 query",
        pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
            0,
            0,
            OrderDirection::Bid,
            sender.clone(),
            Uint128::one(),
            Decimal256::zero(),
            None,
        ))],
        base_denom: base_denom.to_string(),
        quote_denom: quote_denom.to_string(),
        expected_price: Decimal::one(),
        expected_error: None,
    }];

    for test in test_cases {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            quote_denom.to_string(),
            base_denom.to_string(),
        )
        .unwrap();

        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        let res = query_spot_price(deps.as_ref(), test.quote_denom, test.base_denom);

        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                format_test_name(test.name)
            );

            continue;
        }

        let res = res.unwrap();

        assert_eq!(res.spot_price, test.expected_price)
    }
}

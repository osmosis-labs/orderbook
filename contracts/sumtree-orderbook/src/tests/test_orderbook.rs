use crate::{
    constants::{MAX_TICK, MIN_TICK},
    orderbook::*,
    state::ORDERBOOK,
    tests::{
        mock_querier::mock_dependencies_custom,
        test_constants::{MOCK_BASE_DENOM, MOCK_QUOTE_DENOM},
    },
};

#[test]
fn test_create_orderbook() {
    let mut deps = mock_dependencies_custom();

    // Attempt to create an orderbook
    let quote_denom = MOCK_QUOTE_DENOM.to_string();
    let base_denom = MOCK_BASE_DENOM.to_string();
    create_orderbook(deps.as_mut(), quote_denom.clone(), base_denom.clone()).unwrap();

    // Verify orderbook is saved correctly
    let orderbook = ORDERBOOK.load(deps.as_ref().storage).unwrap();
    assert_eq!(orderbook.quote_denom, quote_denom);
    assert_eq!(orderbook.base_denom, base_denom);
    assert_eq!(orderbook.current_tick, 0);
    assert_eq!(orderbook.next_bid_tick, MIN_TICK);
    assert_eq!(orderbook.next_ask_tick, MAX_TICK);
}

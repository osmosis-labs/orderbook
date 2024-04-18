use crate::{
    constants::{MAX_TICK, MIN_TICK},
    orderbook::*,
    state::ORDERBOOK,
};
use cosmwasm_std::testing::mock_dependencies;

#[test]
fn test_create_orderbook() {
    let mut deps = mock_dependencies();

    // Attempt to create an orderbook
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();
    create_orderbook(deps.as_mut(), quote_denom.clone(), base_denom.clone()).unwrap();

    // Verify orderbook is saved correctly
    let orderbook = ORDERBOOK.load(deps.as_ref().storage).unwrap();
    assert_eq!(orderbook.quote_denom, quote_denom);
    assert_eq!(orderbook.base_denom, base_denom);
    assert_eq!(orderbook.current_tick, 0);
    assert_eq!(orderbook.next_bid_tick, MIN_TICK);
    assert_eq!(orderbook.next_ask_tick, MAX_TICK);
}

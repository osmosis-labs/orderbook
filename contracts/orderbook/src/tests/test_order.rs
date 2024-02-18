use crate::error::ContractError;
use crate::order::place_limit;
use crate::orderbook::*;
use crate::state::*;
use crate::types::OrderDirection;
use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, Addr, Uint128};

#[test]
fn test_place_limit_order() {
    let coin_vec = vec![coin(1000, "base")];
    let balances = [("creator", coin_vec.as_slice())];
    let mut deps = mock_dependencies_with_balances(&balances);
    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Create an orderbook first
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();
    let create_response = create_orderbook(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        quote_denom,
        base_denom,
    )
    .unwrap();

    // Retrieve the book_id from the first attribute of create_response
    let book_id: u64 = create_response.attributes[1]
        .value
        .parse()
        .expect("book_id attribute parse error");

    // Parameters for place_limit call
    let tick_id: i64 = 1;
    let order_direction = OrderDirection::Ask;
    let quantity = Uint128::new(100);

    // Assuming order_id starts at 1 for simplicity
    let expected_order_id = 0;

    // Call the place_limit function
    let response = place_limit(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        book_id,
        tick_id,
        order_direction.clone(),
        quantity,
    )
    .unwrap();

    // Assertions on the response
    assert_eq!(response.attributes[0], ("method", "placeLimit"));
    assert_eq!(response.attributes[1], ("owner", "creator"));
    assert_eq!(response.attributes[2], ("book_id", book_id.to_string()));
    assert_eq!(response.attributes[3], ("tick_id", tick_id.to_string()));
    assert_eq!(
        response.attributes[4],
        ("order_id", expected_order_id.to_string())
    );
    assert_eq!(
        response.attributes[5],
        ("order_direction", format!("{:?}", order_direction))
    );
    assert_eq!(response.attributes[6], ("quantity", quantity.to_string()));

    // Retrieve the order from storage to verify it was saved correctly
    let order = orders()
        .load(&deps.storage, &(book_id, tick_id, expected_order_id))
        .unwrap();

    // Verify the order's fields
    assert_eq!(order.book_id, book_id);
    assert_eq!(order.tick_id, tick_id);
    assert_eq!(order.order_id, expected_order_id);
    assert_eq!(order.order_direction, order_direction);
    assert_eq!(order.owner, Addr::unchecked("creator"));
    assert_eq!(order.quantity, Uint128::new(100));
}

#[test]
fn test_place_limit_with_invalid_book_id() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Create an orderbook first
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();
    let create_response = create_orderbook(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        quote_denom,
        base_denom,
    )
    .unwrap();

    // Retrieve the book_id from the first attribute of create_response
    let valid_book_id: u64 = create_response.attributes[1]
        .value
        .parse()
        .expect("book_id attribute parse error");

    let invalid_book_id = valid_book_id + 1;

    let response = place_limit(
        deps.as_mut(),
        env,
        info,
        invalid_book_id,
        1, // tick_id
        OrderDirection::Ask,
        Uint128::new(100),
    );

    assert!(matches!(
        response,
        Err(ContractError::InvalidBookId { book_id }) if book_id == invalid_book_id
    ));
}

#[test]
fn test_place_limit_with_invalid_tick_id() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Create an orderbook first
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();
    let create_response = create_orderbook(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        quote_denom,
        base_denom,
    )
    .unwrap();

    // Retrieve the book_id from the first attribute of create_response
    let book_id: u64 = create_response.attributes[1]
        .value
        .parse()
        .expect("book_id attribute parse error");

    let invalid_tick_id = MAX_TICK + 1;

    let response = place_limit(
        deps.as_mut(),
        env,
        info,
        book_id,
        invalid_tick_id,
        OrderDirection::Ask,
        Uint128::new(100),
    );

    assert!(matches!(
        response,
        Err(ContractError::InvalidTickId { tick_id }) if tick_id == invalid_tick_id
    ));
}

#[test]
fn test_place_limit_with_invalid_quantity() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Create an orderbook first
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();
    let create_response = create_orderbook(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        quote_denom,
        base_denom,
    )
    .unwrap();

    // Retrieve the book_id from the first attribute of create_response
    let book_id: u64 = create_response.attributes[1]
        .value
        .parse()
        .expect("book_id attribute parse error");

    let invalid_quantity = Uint128::zero(); // Invalid quantity

    let response = place_limit(
        deps.as_mut(),
        env,
        info,
        book_id,
        1, // tick_id
        OrderDirection::Ask,
        invalid_quantity,
    );

    assert!(matches!(
        response,
        Err(ContractError::InvalidQuantity { quantity }) if quantity == invalid_quantity
    ));
}

#[test]
fn test_place_limit_with_insufficient_funds() {
    let insufficient_balance = Uint128::new(500); // Mocked balance less than required
    let balances = vec![(coin(insufficient_balance.u128(), "base"))];
    let mut deps = mock_dependencies_with_balances(&[("creator", &balances)]);

    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Create an orderbook first
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();
    let create_response = create_orderbook(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        quote_denom,
        base_denom,
    )
    .unwrap();

    // Retrieve the book_id from the first attribute of create_response
    let book_id: u64 = create_response.attributes[1]
        .value
        .parse()
        .expect("book_id attribute parse error");

    let required_quantity = Uint128::new(1000); // Quantity greater than the mocked balance

    let response = place_limit(
        deps.as_mut(),
        env,
        info,
        book_id,
        1, // tick_id
        OrderDirection::Ask,
        required_quantity,
    );

    assert!(matches!(
        response,
        Err(ContractError::InsufficientFunds { balance, required }) if balance == insufficient_balance && required == required_quantity
    ));
}

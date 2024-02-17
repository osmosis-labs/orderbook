use crate::state::*;
use crate::types::{FilterOwnerOrders, LimitOrder, OrderDirection};
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Order, Uint128};

#[test]
fn test_new_orderbook_id() {
    let mut storage = MockStorage::new();
    let id = new_orderbook_id(&mut storage).unwrap();
    assert_eq!(id, 0);
    let id = new_orderbook_id(&mut storage).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_order_id_works() {
    let mut storage = MockStorage::new();
    let id = new_order_id(&mut storage).unwrap();
    assert_eq!(id, 0);
    let id = new_order_id(&mut storage).unwrap();
    assert_eq!(id, 1);
}

#[test]
fn test_tick_iteration() {
    let mut storage = MockStorage::new();
    let book_id = new_orderbook_id(&mut storage).unwrap();
    let tick_amount = 50;
    for i in -tick_amount..tick_amount {
        TICK_LIQUIDITY
            .save(&mut storage, &(book_id, i), &Uint128::new(i as u128))
            .unwrap();
    }
    let prefix = TICK_LIQUIDITY.prefix(book_id);
    let ticks_asc: Vec<i64> = prefix
        .keys(&storage, None, None, Order::Ascending)
        .map(|result| result.unwrap())
        .collect();
    let ticks_desc: Vec<i64> = prefix
        .keys(&storage, None, None, Order::Descending)
        .map(|result| result.unwrap())
        .collect();
    for i in 0..tick_amount * 2 {
        assert_eq!(ticks_asc[i as usize], -tick_amount + i);
        assert_eq!(ticks_desc[i as usize], tick_amount - i - 1);
    }
}

#[test]
fn test_order_iteration() {
    let mut storage = MockStorage::new();
    let order_amount = 50;
    let book_id = new_orderbook_id(&mut storage).unwrap();
    let tick = 0;
    for i in 0..order_amount {
        let order_id = new_order_id(&mut storage).unwrap();
        let order = LimitOrder {
            tick_id: tick,
            book_id,
            order_id,
            owner: Addr::unchecked(format!("maker{i}")),
            quantity: Uint128::new(i as u128),
            order_direction: OrderDirection::Ask,
        };
        orders()
            .save(&mut storage, &(book_id, tick, i), &order)
            .unwrap();
    }

    let tick_orders = orders().prefix((book_id, tick));
    let orders_desc: Vec<LimitOrder> = tick_orders
        .range(&storage, None, None, Order::Descending)
        .map(|result| result.unwrap().1)
        .collect();
    let orders_asc: Vec<LimitOrder> = tick_orders
        .range(&storage, None, None, Order::Ascending)
        .map(|result| result.unwrap().1)
        .collect();
    for i in 0..order_amount {
        assert_eq!(orders_desc[i as usize].order_id, order_amount - i - 1);
        assert_eq!(orders_asc[i as usize].order_id, i);
    }
}

#[test]
fn test_get_orders_by_owner_all() {
    let mut storage = MockStorage::new();
    let order_amount = 10;
    let owner = "owner1";

    let book_ids: Vec<u64> = (0..3)
        .map(|_| new_orderbook_id(&mut storage).unwrap())
        .collect();

    (0..order_amount).for_each(|i| {
        let order_id = new_order_id(&mut storage).unwrap();
        let other_owner = &format!("owner{i}");
        let current_owner = Addr::unchecked(if i % 2 == 0 { owner } else { other_owner });
        let order = LimitOrder::new(
            book_ids[i % 3],
            0,
            order_id,
            OrderDirection::Ask,
            current_owner,
            Uint128::new(i as u128),
        );
        orders()
            .save(&mut storage, &(order.book_id, 0, i as u64), &order)
            .unwrap();
    });

    let owner_orders: Vec<LimitOrder> =
        get_orders_by_owner(&storage, FilterOwnerOrders::All(Addr::unchecked(owner))).unwrap();

    assert_eq!(owner_orders.len(), order_amount / 2 + 1);
    owner_orders.iter().for_each(|order| {
        assert_eq!(order.owner, Addr::unchecked(owner));
    });
}

#[test]
fn test_get_orders_by_owner_by_book() {
    let mut storage = MockStorage::new();
    let order_amount = 100;
    let owner = "owner1";

    // Generate three new book IDs
    let book_ids: Vec<u64> = (0..3)
        .map(|_| new_orderbook_id(&mut storage).unwrap())
        .collect();

    // Create orders alternating ownership between `owner` and dynamically generated owners amongst all books evenly
    (0..order_amount).for_each(|i| {
        let order_id = new_order_id(&mut storage).unwrap();
        let other_owner = &format!("owner{i}");
        let current_owner = Addr::unchecked(if i % 2 == 0 { owner } else { other_owner });
        let order = LimitOrder::new(
            book_ids[i % 3],
            0,
            order_id,
            OrderDirection::Ask,
            current_owner,
            Uint128::new(i as u128),
        );
        orders()
            .save(&mut storage, &(order.book_id, 0, i as u64), &order)
            .unwrap();
    });

    // Verify orders by book ID
    book_ids.iter().for_each(|&book_id| {
        let owner_orders = get_orders_by_owner(
            &storage,
            FilterOwnerOrders::ByBook(book_id, Addr::unchecked(owner)),
        )
        .unwrap();
        assert!(!owner_orders.is_empty());
        owner_orders.iter().for_each(|order| {
            assert_eq!(order.owner, Addr::unchecked(owner));
            assert_eq!(order.book_id, book_id);
        });
    });
}

#[test]
fn test_get_orders_by_owner_by_tick() {
    let mut storage = MockStorage::new();
    let order_amount = 100;
    let ticks = [0, 1, 2];
    let owner = "owner1";
    let book_id = new_orderbook_id(&mut storage).unwrap();

    // Create orders alternating ownership between `owner` and dynamically generated owners amongst all ticks evenly
    (0..order_amount).for_each(|i| {
        let order_id = new_order_id(&mut storage).unwrap();
        let other_owner = &format!("owner{i}");
        let current_owner = Addr::unchecked(if i % 2 == 0 { owner } else { other_owner });
        let tick = ticks[i % 3];
        let order = LimitOrder::new(
            book_id,
            tick,
            order_id,
            OrderDirection::Ask,
            current_owner,
            Uint128::new(i as u128),
        );
        orders()
            .save(&mut storage, &(book_id, tick, i as u64), &order)
            .unwrap();
    });

    ticks.iter().for_each(|&tick| {
        let owner_orders = get_orders_by_owner(
            &storage,
            FilterOwnerOrders::ByTick(book_id, tick, Addr::unchecked(owner)),
        )
        .unwrap();
        assert!(!owner_orders.is_empty());
        owner_orders.iter().for_each(|order| {
            assert_eq!(order.owner, Addr::unchecked(owner));
            assert_eq!(order.tick_id, tick);
        });
    });
}

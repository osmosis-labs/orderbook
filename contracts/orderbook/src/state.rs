use crate::types::{FilterOwnerOrders, LimitOrder, Orderbook};
use crate::ContractError;
use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

pub const MIN_TICK: i64 = -108000000;
pub const MAX_TICK: i64 = 342000000;

pub const ORDERBOOKS: Map<&u64, Orderbook> = Map::new("orderbooks");
/// Key: (orderbook_id, tick)
pub const TICK_LIQUIDITY: Map<&(u64, i64), Uint128> = Map::new("tick_liquidity");

// TODO: Check additional gas fee for adding more indexes
pub struct OrderIndexes {
    // Index by owner; Generic types: MultiIndex<Index Key: owner, Input Data: LimitOrder, Map Key: (orderbook_id, tick, order_id)>
    pub owner: MultiIndex<'static, Addr, LimitOrder, (u64, i64, u64)>,
    // Index by book and owner; Generic types: MultiIndex<Index Key: (book_id, owner), Input Data: LimitOrder, Map Key: (orderbook_id, tick, order_id)>
    pub book_and_owner: MultiIndex<'static, (u64, Addr), LimitOrder, (u64, i64, u64)>,
    // Index by tick and owner; Generic types: MultiIndex<Index Key: (book_id, tick_id, owner), Input Data: LimitOrder, Map Key: (orderbook_id, tick, order_id)>
    pub tick_and_owner: MultiIndex<'static, (u64, i64, Addr), LimitOrder, (u64, i64, u64)>,
}

impl IndexList<LimitOrder> for OrderIndexes {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<LimitOrder>> + '_> {
        let v: Vec<&dyn Index<LimitOrder>> =
            vec![&self.owner, &self.book_and_owner, &self.tick_and_owner];
        Box::new(v.into_iter())
    }
}

/// Key: (orderbook_id, tick, order_id)
pub fn orders() -> IndexedMap<'static, &'static (u64, i64, u64), LimitOrder, OrderIndexes> {
    IndexedMap::new(
        "orders",
        OrderIndexes {
            owner: MultiIndex::new(
                |_, d: &LimitOrder| d.owner.clone(),
                "orders",
                "orders_owner",
            ),
            book_and_owner: MultiIndex::new(
                |_, d: &LimitOrder| (d.book_id, d.owner.clone()),
                "orders",
                "orders_book_and_owner",
            ),
            tick_and_owner: MultiIndex::new(
                |_, d: &LimitOrder| (d.book_id, d.tick_id, d.owner.clone()),
                "orders",
                "orders_tick_and_owner",
            ),
        },
    )
}

// Counters for ID tracking
pub const ORDER_ID: Item<u64> = Item::new("order_id");
pub const ORDERBOOK_ID: Item<u64> = Item::new("orderbook_id");

pub fn new_orderbook_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id = ORDERBOOK_ID.load(storage).unwrap_or_default();
    ORDERBOOK_ID.save(storage, &(id + 1))?;
    Ok(id)
}

pub fn new_order_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id = ORDER_ID.load(storage).unwrap_or_default();
    ORDER_ID.save(storage, &(id + 1))?;
    Ok(id)
}

// TODO: Add pagination
// TODO: How finite do we need queries?

/// Retrieves a list of `LimitOrder` filtered by the specified `FilterOwnerOrders`.
pub fn get_orders_by_owner(
    storage: &dyn Storage,
    filter: FilterOwnerOrders,
) -> StdResult<Vec<LimitOrder>> {
    let orders: Vec<LimitOrder> = match filter {
        FilterOwnerOrders::All(owner) => orders()
            .idx
            .owner
            .prefix(owner)
            .range(storage, None, None, Order::Ascending)
            .filter_map(|item| item.ok())
            .map(|(_, order)| order)
            .collect(),
        FilterOwnerOrders::ByBook(book_id, owner) => orders()
            .idx
            .book_and_owner
            .prefix((book_id, owner))
            .range(storage, None, None, Order::Ascending)
            .filter_map(|item| item.ok())
            .map(|(_, order)| order)
            .collect(),
        FilterOwnerOrders::ByTick(book_id, tick_id, owner) => orders()
            .idx
            .tick_and_owner
            .prefix((book_id, tick_id, owner))
            .range(storage, None, None, Order::Ascending)
            .filter_map(|item| item.ok())
            .map(|(_, order)| order)
            .collect(),
    };
    Ok(orders)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::OrderDirection;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{Addr, Order};

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
}

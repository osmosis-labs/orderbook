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

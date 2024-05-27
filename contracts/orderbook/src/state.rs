use crate::types::{FilterOwnerOrders, LimitOrder, Orderbook};
use crate::ContractError;
use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

// Counters for ID tracking
pub const ORDER_ID: Item<u64> = Item::new("order_id");
pub const ORDERBOOK_ID: Item<u64> = Item::new("orderbook_id");

// Pagination constants for queries
const MAX_PAGE_SIZE: u8 = 100;
const DEFAULT_PAGE_SIZE: u8 = 50;

pub const ORDERBOOKS: Map<&u64, Orderbook> = Map::new("orderbooks");
/// Key: (orderbook_id, tick)
pub const TICK_LIQUIDITY: Map<&(u64, i64), Uint128> = Map::new("tick_liquidity");

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

/// Reduces the liquidity of a tick by the specified amount and removes it if no liquidity remains.
pub fn reduce_tick_liquidity(
    storage: &mut dyn Storage,
    book_id: u64,
    tick_id: i64,
    amount: Uint128,
) -> Result<(), ContractError> {
    let tick_liquidity = TICK_LIQUIDITY
        .may_load(storage, &(book_id, tick_id))?
        .ok_or(ContractError::InvalidTickId { tick_id })?;
    let new_liquidity = tick_liquidity.checked_sub(amount)?;
    if new_liquidity.is_zero() {
        TICK_LIQUIDITY.remove(storage, &(book_id, tick_id));
    } else {
        TICK_LIQUIDITY.save(storage, &(book_id, tick_id), &new_liquidity)?;
    };
    Ok(())
}

/// Retrieves a list of `LimitOrder` filtered by the specified `FilterOwnerOrders`.
///
/// This function allows for filtering orders based on the owner's address, optionally further
/// filtering by book ID or tick ID. It supports pagination through `min`, `max`, and `page_size` parameters.
///
/// ## Arguments
///
/// * `storage` - CosmWasm Storage struct
/// * `filter` - Specifies how to filter orders based on the owner. Can be by all orders of the owner,
/// by a specific book, or by a specific tick within a book.
/// * `min` - An optional minimum bound (exclusive) for the order key (orderbook_id, tick, order_id) to start the query.
/// * `max` - An optional maximum bound (exclusive) for the order key to end the query.
/// * `page_size` - An optional maximum number of orders to return. Limited by `MAX_PAGE_SIZE = 100` defaults to `DEFAULT_PAGE_SIZE = 50`.
///
/// ## Returns
///
/// A result containing either a vector of `LimitOrder` matching the criteria or an error.
pub fn get_orders_by_owner(
    storage: &dyn Storage,
    filter: FilterOwnerOrders,
    min: Option<(u64, i64, u64)>,
    max: Option<(u64, i64, u64)>,
    page_size: Option<u8>,
) -> StdResult<Vec<LimitOrder>> {
    let page_size = page_size.map(|page_size| page_size.min(MAX_PAGE_SIZE));
    let min = min.map(Bound::exclusive);
    let max = max.map(Bound::exclusive);

    // Define the prefix iterator based on the filter
    let iter = match filter {
        FilterOwnerOrders::All(owner) => orders().idx.owner.prefix(owner),
        FilterOwnerOrders::ByBook(book_id, owner) => {
            orders().idx.book_and_owner.prefix((book_id, owner))
        }
        FilterOwnerOrders::ByTick(book_id, tick_id, owner) => orders()
            .idx
            .tick_and_owner
            .prefix((book_id, tick_id, owner)),
    };

    let order_iter = iter.range(storage, min, max, Order::Ascending);

    // Get orders based on pagination
    let orders = if let Some(page_size) = page_size {
        order_iter
            .take(page_size as usize)
            .filter_map(|item| item.ok())
            .map(|(_, order)| order)
            .collect()
    } else {
        order_iter
            .filter_map(|item| item.ok())
            .map(|(_, order)| order)
            .collect()
    };

    Ok(orders)
}

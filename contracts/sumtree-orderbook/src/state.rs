use crate::error::ContractResult;
use crate::types::{FilterOwnerOrders, LimitOrder, OrderDirection, Orderbook, TickState};
use crate::ContractError;
use cosmwasm_std::{Addr, Decimal256, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

// Counters for ID tracking
pub const ORDER_ID: Item<u64> = Item::new("order_id");

// Pagination constants for queries
const MAX_PAGE_SIZE: u8 = 100;
const DEFAULT_PAGE_SIZE: u8 = 50;

pub const ORDERBOOK: Item<Orderbook> = Item::new("orderbook");
pub const TICK_STATE: Map<i64, TickState> = Map::new("tick_state");
pub const DIRECTION_TOTAL_LIQUIDITY: Map<&str, Decimal256> = Map::new("direction_liquidity");
pub const IS_ACTIVE: Item<bool> = Item::new("is_active");

pub struct OrderIndexes {
    // Index by owner; Generic types: MultiIndex<Index Key: owner, Input Data: LimitOrder, Map Key: ( tick, order_id)>
    pub owner: MultiIndex<'static, Addr, LimitOrder, (i64, u64)>,
    // Index by tick and owner; Generic types: MultiIndex<Index Key: (tick_id, owner), Input Data: LimitOrder, Map Key: (tick, order_id)>
    pub tick_and_owner: MultiIndex<'static, (i64, Addr), LimitOrder, (i64, u64)>,
}

impl IndexList<LimitOrder> for OrderIndexes {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<LimitOrder>> + '_> {
        let v: Vec<&dyn Index<LimitOrder>> = vec![&self.owner, &self.tick_and_owner];
        Box::new(v.into_iter())
    }
}

/// Key: (tick_id, order_id)
pub fn orders() -> IndexedMap<'static, &'static (i64, u64), LimitOrder, OrderIndexes> {
    IndexedMap::new(
        "orders",
        OrderIndexes {
            owner: MultiIndex::new(
                |_, d: &LimitOrder| d.owner.clone(),
                "orders",
                "orders_owner",
            ),
            tick_and_owner: MultiIndex::new(
                |_, d: &LimitOrder| (d.tick_id, d.owner.clone()),
                "orders",
                "orders_tick_and_owner",
            ),
        },
    )
}

pub fn new_order_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id = ORDER_ID.load(storage).unwrap_or_default();
    ORDER_ID.save(storage, &(id + 1))?;
    Ok(id)
}

/// Retrieves a list of `LimitOrder` filtered by the specified `FilterOwnerOrders`.
///
/// This function allows for filtering orders based on the owner's address, optionally further
/// filtering by tick ID. It supports pagination through `min`, `max`, and `page_size` parameters.
///
/// ## Arguments
///
/// * `storage` - CosmWasm Storage struct
/// * `filter` - Specifies how to filter orders based on the owner. Can be by all orders of the owner,
/// by a specific book, or by a specific tick within a book.
/// * `min` - An optional minimum bound (exclusive) for the order key (tick, order_id) to start the query.
/// * `max` - An optional maximum bound (exclusive) for the order key to end the query.
/// * `page_size` - An optional maximum number of orders to return. Limited by `MAX_PAGE_SIZE = 100` defaults to `DEFAULT_PAGE_SIZE = 50`.
///
/// ## Returns
///
/// A result containing either a vector of `LimitOrder` matching the criteria or an error.
pub fn get_orders_by_owner(
    storage: &dyn Storage,
    filter: FilterOwnerOrders,
    min: Option<(i64, u64)>,
    max: Option<(i64, u64)>,
    page_size: Option<u8>,
) -> StdResult<Vec<LimitOrder>> {
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE) as usize;
    let min = min.map(Bound::exclusive);
    let max = max.map(Bound::exclusive);

    // Define the prefix iterator based on the filter
    let iter = match filter {
        FilterOwnerOrders::All(owner) => orders().idx.owner.prefix(owner),
        FilterOwnerOrders::ByTick(tick_id, owner) => {
            orders().idx.tick_and_owner.prefix((tick_id, owner))
        }
    };

    // Get orders based on pagination
    let orders: Vec<LimitOrder> = iter
        .range(storage, min, max, Order::Ascending)
        .take(page_size)
        .filter_map(|item| item.ok())
        .map(|(_, order)| order)
        .collect();

    Ok(orders)
}

/// Gets the currently stored total liquidity for the specified `OrderDirection`.
///
/// Defaults to 0 for empty values.
pub fn get_directional_liquidity(
    storage: &dyn Storage,
    direction: OrderDirection,
) -> ContractResult<Decimal256> {
    let direction_key = &direction.to_string();
    let current_liquidity = DIRECTION_TOTAL_LIQUIDITY
        .load(storage, direction_key)
        .unwrap_or_default();
    Ok(current_liquidity)
}

/// Adds the specified amount of liquidity to the specified `OrderDirection`'s total liquidity.
///
/// Errors on Decimal256 overflow.
pub fn add_directional_liquidity(
    storage: &mut dyn Storage,
    direction: OrderDirection,
    amount: Decimal256,
) -> ContractResult<()> {
    let direction_key = &direction.to_string();
    let current_liquidity = DIRECTION_TOTAL_LIQUIDITY
        .load(storage, direction_key)
        .unwrap_or_default();
    DIRECTION_TOTAL_LIQUIDITY.save(
        storage,
        direction_key,
        &(current_liquidity.checked_add(amount)?),
    )?;
    Ok(())
}

/// Subtracts the specified amount of liquidity from the specified `OrderDirection`'s total liquidity.
///
/// Errors on Decimal256 underflow.
pub fn subtract_directional_liquidity(
    storage: &mut dyn Storage,
    direction: OrderDirection,
    amount: Decimal256,
) -> ContractResult<()> {
    let direction_key = &direction.to_string();
    let current_liquidity = DIRECTION_TOTAL_LIQUIDITY
        .load(storage, direction_key)
        .unwrap_or_default();
    DIRECTION_TOTAL_LIQUIDITY.save(
        storage,
        direction_key,
        &(current_liquidity.checked_sub(amount)?),
    )?;
    Ok(())
}

use cosmwasm_std::{Addr, Storage, Uint128};
use cw_storage_plus::{Map, Item};
use crate::ContractError;
use crate::orderbook::types::{LimitOrder, Orderbook};

// TODO: Maybe unnecessary
pub const OWNER: Item<Addr> = Item::new("owner");
pub const ORDER_BOOKS: Map<u64, Orderbook> = Map::new("order_books");
// Key: (order_book_id, tick)
pub const TICK_LIQUIDITY: Map<(u64, i64), Uint128> = Map::new("tick_liquidity");
// Key: (order_book_id, tick, order_id)
pub const ORDERS: Map<(u64, i64, u64), LimitOrder> = Map::new("tick_orders");

// Counters for ID tracking
// TODO: Do order IDs need to be unique across all order books? We could store this as a map between order book id and the current counter, or a string amalgamation of the two
pub const ORDER_ID: Item<u64> = Item::new("order_id");
pub const ORDER_BOOK_ID: Item<u64> = Item::new("order_book_id");

pub fn new_order_book_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id = ORDER_BOOK_ID.load(storage).unwrap_or_default();
    ORDER_BOOK_ID.save(storage, &(id + 1))?;
    Ok(id)
}

pub fn new_order_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id = ORDER_ID.load(storage).unwrap_or_default();
    ORDER_ID.save(storage, &(id + 1))?;
    Ok(id)
}

// Alternate implementation if we decide to make IDs unique per order book
// pub fn new_order_id(storage: &mut dyn Storage, book_id: u64) -> Result<u64, ContractError> {
//     let id = ORDER_ID.load(storage, book_id).unwrap_or_default();
//     ORDER_ID.save(storage, book_id,&(id + 1)).unwrap();
//     Ok(id)
// }

#[cfg(test)]
mod test {
    use cosmwasm_std::{Order};
    use super::*;
    use cosmwasm_std::testing::MockStorage;
    use crate::orderbook::types::OrderDirection;

    #[test]
    fn test_new_order_book_id() {
        let mut storage = MockStorage::new();
        let id = new_order_book_id(&mut storage).unwrap();
        assert_eq!(id, 0);
        let id = new_order_book_id(&mut storage).unwrap();
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
        let book_id = new_order_book_id(&mut storage).unwrap();
        let tick_amount = 50;
        for i in -tick_amount..tick_amount {
            TICK_LIQUIDITY.save(&mut storage, (book_id, i), &Uint128::new(i as u128)).unwrap();
        }
        let prefix = TICK_LIQUIDITY.prefix(book_id);
        let ticks_asc: Vec<i64> = prefix.keys(&storage, None, None, Order::Ascending).map(|result| result.unwrap()).collect();
        let ticks_desc: Vec<i64> = prefix.keys(&storage, None, None, Order::Descending).map(|result| result.unwrap()).collect();
        for i in 0..tick_amount*2 {
            assert_eq!(ticks_asc[i as usize], -tick_amount + i);
            assert_eq!(ticks_desc[i as usize], tick_amount - i - 1);
        }
    }

    #[test]
    fn test_order_iteration() {
        let mut storage = MockStorage::new();
        let order_amount = 50;
        let book_id = new_order_book_id(&mut storage).unwrap();
        let tick = 0;
        for i in 0..order_amount {
            let order_id = new_order_id(&mut storage).unwrap();
            let order = LimitOrder {
                tick_id: tick,
                book_id,
                order_id,
                owner: Addr::unchecked(format!("maker{}", i)),
                quantity: i,
                order_direction: OrderDirection::Ask,
            };
            ORDERS.save(&mut storage, (book_id, tick, i), &order).unwrap();
        }

        let tick_orders = ORDERS.prefix((book_id, tick));
        let orders_desc: Vec<LimitOrder> = tick_orders.range(&storage, None, None, Order::Descending).map(|result| result.unwrap().1).collect();
        let orders_asc: Vec<LimitOrder> = tick_orders.range(&storage, None, None, Order::Ascending).map(|result| result.unwrap().1).collect();
        for i in 0..order_amount {
            assert_eq!(orders_desc[i as usize].order_id, order_amount - i - 1);
            assert_eq!(orders_asc[i as usize].order_id, i);
        }
    }
}
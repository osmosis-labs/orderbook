use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid tick ID: {tick_id:?}")]
    InvalidTickId { tick_id: i64 },

    #[error("Invalid quantity: {quantity:?}")]
    InvalidQuantity { quantity: Uint128 },

    #[error("Insufficient funds. Balance: {balance:?}, Required: {required:?}")]
    InsufficientFunds { balance: Uint128, required: Uint128 },

    #[error("Invalid book ID: {book_id:?}")]
    InvalidBookId { book_id: u64 },
}

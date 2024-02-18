use cosmwasm_std::{CoinsError, StdError, Uint128};
use cw_utils::PaymentError;
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

    #[error("Insufficient funds. Sent: {sent:?}, Required: {required:?}")]
    InsufficientFunds { sent: Uint128, required: Uint128 },

    #[error("Invalid book ID: {book_id:?}")]
    InvalidBookId { book_id: u64 },

    #[error(transparent)]
    Coins(#[from] CoinsError),

    #[error(transparent)]
    PaymentError(#[from] PaymentError),
}

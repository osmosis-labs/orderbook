use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, CoinsError, ConversionOverflowError, Decimal,
    DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError, Uint128,
};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

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

    #[error("Order not found: {book_id:?}, {tick_id:?}, {order_id:?}")]
    OrderNotFound {
        book_id: u64,
        tick_id: i64,
        order_id: u64,
    },

    #[error("Reply error: {id:?}, {error:?}")]
    ReplyError { id: u64, error: String },

    // Decimal-related errors
    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    // Tick out of bounds error
    #[error("Tick out of bounds: {tick_id:?}")]
    TickOutOfBounds { tick_id: i64 },
    #[error("Cannot fulfill order. Order ID: {order_id:?}, Book ID: {book_id:?}, Amount Required: {amount_required:?}, Amount Remaining: {amount_remaining:?} {reason:?}")]
    InvalidFulfillment {
        order_id: u64,
        book_id: u64,
        amount_required: Uint128,
        amount_remaining: Uint128,
        reason: Option<String>,
    },

    #[error("Mismatched order direction")]
    MismatchedOrderDirection {},

    #[error("Invalid Node Type")]
    InvalidNodeType,

    #[error("Childless Internal Node")]
    ChildlessInternalNode,

    #[error("Cannot cancel an order that has partially or fully been filled")]
    CancelFilledOrder,

    #[error("Invalid tick state: syncing tick pushed ETAS past CTT")]
    InvalidTickSync,

    #[error("Zero Claim: Nothing to be claimed yet")]
    ZeroClaim,

    #[error("Node insertion error")]
    NodeInsertionError,

    #[error("Auto claim bounty must be a value between 0 and 1. Received: {claim_bounty:?}")]
    InvalidAutoClaimBounty { claim_bounty: Option<Decimal> },
}

pub type ContractResult<T> = Result<T, ContractError>;

use crate::constants::MAX_MAKER_FEE_PERCENTAGE;
use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, CoinsError, ConversionOverflowError,
    Decimal256, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError, Uint128,
};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Coins(#[from] CoinsError),

    #[error(transparent)]
    PaymentError(#[from] PaymentError),

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

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid tick ID: {tick_id:?}")]
    InvalidTickId { tick_id: i64 },

    #[error("Invalid quantity: {quantity:?}")]
    InvalidQuantity { quantity: Uint128 },

    #[error("Insufficient funds. Sent: {sent:?}, Required: {required:?}")]
    InsufficientFunds { sent: Uint128, required: Uint128 },

    #[error("Invalid pair: ({token_in_denom}, {token_out_denom})")]
    InvalidPair {
        token_in_denom: String,
        token_out_denom: String,
    },

    #[error("Invalid swap: {error}")]
    InvalidSwap { error: String },

    #[error("Invalid denom")]
    InvalidDenom { denom: String },

    #[error("Order not found: {tick_id:?}, {order_id:?}")]
    OrderNotFound { tick_id: i64, order_id: u64 },

    #[error("Reply error: {id:?}, {error:?}")]
    ReplyError { id: u64, error: String },

    // Tick out of bounds error
    #[error("Tick out of bounds: {tick_id:?}")]
    TickOutOfBounds { tick_id: i64 },

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

    #[error("Orderbook ran out of liquidity during market order")]
    InsufficientLiquidity,

    #[error("Claim bounty must be a value between 0 and 0.01 (1%). Received: {claim_bounty:?}")]
    InvalidClaimBounty { claim_bounty: Option<Decimal256> },

    #[error(
        "Exceeded the maximum number of claims in a batch. Maximum allowed: {max_batch_claim:?}"
    )]
    BatchClaimLimitExceeded { max_batch_claim: u32 },

    #[error("Orderbook is inactive")]
    Inactive,

    #[error("No maker fee recipient currently set")]
    NoMakerFeeRecipient,

    #[error("Invalid Maker Fee Recipient")]
    InvalidMakerFeeRecipient,

    #[error("Invalid Maker Fee: provided fee must be less than or equal to {MAX_MAKER_FEE_PERCENTAGE:?}")]
    InvalidMakerFee,
}

pub type ContractResult<T> = Result<T, ContractError>;

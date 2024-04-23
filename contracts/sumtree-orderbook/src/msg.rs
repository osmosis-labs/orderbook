use crate::types::OrderDirection;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub base_denom: String,
    pub quote_denom: String,
    pub admin: Addr,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    PlaceLimit {
        tick_id: i64,
        order_direction: OrderDirection,
        quantity: Uint128,
        claim_bounty: Option<Decimal>,
    },
    CancelLimit {
        tick_id: i64,
        order_id: u64,
    },
    PlaceMarket {
        order_direction: OrderDirection,
        quantity: Uint128,
    },
    ClaimLimit {
        tick_id: i64,
        order_id: u64,
    },
    BatchClaim {
        orders: Vec<(i64, u64)>,
    },
    Auth(AuthExecuteMsg),
}

#[cw_serde]
pub enum AuthExecuteMsg {
    // -- Admin Messages --
    TransferAdmin { new_admin: Addr },
    CancelAdminTransfer {},
    RejectAdminTransfer {},
    ClaimAdmin {},
    RenounceAdminship {},

    // -- Moderator Messages --
    OfferModerator { new_moderator: Addr },
    RejectModeratorOffer {},
    ClaimModerator {},
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(SpotPriceResponse)]
    SpotPrice {
        quote_asset_denom: String,
        base_asset_denom: String,
    },
    #[returns(CalcOutAmtGivenInResponse)]
    CalcOutAmountGivenIn {
        token_in: Coin,
        token_out_denom: String,
        swap_fee: Decimal,
    },
    #[returns(GetTotalPoolLiquidityResponse)]
    GetTotalPoolLiquidity {},
    /// NO-OP QUERY
    #[returns(CalcInAmtGivenOutResponse)]
    CalcInAmtGivenOut {},

    #[returns(Option<Addr>)]
    Auth(AuthQueryMsg),
}

#[cw_serde]
pub enum AuthQueryMsg {
    Admin {},
    AdminOffer {},

    Moderator {},
    ModeratorOffer {},
}

#[cw_serde]
pub struct SpotPriceResponse {
    pub spot_price: Decimal,
}

#[cw_serde]
pub struct CalcOutAmtGivenInResponse {
    pub token_out: Coin,
}

#[cw_serde]
pub struct CalcInAmtGivenOutResponse {
    pub token_in: Coin,
}

#[cw_serde]
pub struct GetTotalPoolLiquidityResponse {
    pub total_pool_liquidity: Vec<Coin>,
}

#[cw_serde]
pub enum SudoMsg {
    /// SwapExactAmountIn swaps an exact amount of tokens in for as many tokens out as possible.
    /// The amount of tokens out is determined by the current exchange rate and the swap fee.
    /// The user specifies a minimum amount of tokens out, and the transaction will revert if that amount of tokens
    /// is not received.
    SwapExactAmountIn {
        sender: String,
        token_in: Coin,
        token_out_denom: String,
        token_out_min_amount: Uint128,
        swap_fee: Decimal,
    },
    /// SwapExactAmountOut swaps as many tokens in as possible for an exact amount of tokens out.
    /// The amount of tokens in is determined by the current exchange rate and the swap fee.
    /// The user specifies a maximum amount of tokens in, and the transaction will revert if that amount of tokens
    /// is exceeded.
    ///
    /// **Currently this message is no-op**
    SwapExactAmountOut {
        sender: String,
        token_in_denom: String,
        token_in_max_amount: Uint128,
        token_out: Coin,
        swap_fee: Decimal,
    },
    TransferAdmin {
        new_admin: Addr,
    },
    RemoveAdmin {},
}

#[cw_serde]
/// Fixing token in amount makes token amount out varies
pub struct SwapExactAmountInResponseData {
    pub token_out_amount: Uint128,
}

#[cw_serde]
/// Fixing token out amount makes token amount in varies
pub struct SwapExactAmountOutResponseData {
    pub token_in_amount: Uint128,
}

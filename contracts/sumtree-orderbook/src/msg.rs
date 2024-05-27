use crate::types::{OrderDirection, TickState};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, Uint128, Uint256};
use osmosis_std::types::cosmos::base::v1beta1::Coin as ProtoCoin;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub base_denom: String,
    pub quote_denom: String,
    pub admin: Addr,
    pub moderator: Addr,
    pub maker_fee: Option<Decimal>,
    pub maker_fee_recipient: Addr,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    PlaceLimit {
        tick_id: i64,
        order_direction: OrderDirection,
        quantity: Uint128,
        claim_bounty: Option<Decimal256>,
    },
    CancelLimit {
        tick_id: i64,
        order_id: u64,
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

    // -- Shared messages --
    SetActive { active: bool },
    SetMakerFee { fee: Decimal256 },
    SetMakerFeeRecipient { recipient: Addr },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // -- CW Pool Queries --
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
    #[returns(GetSwapFeeResponse)]
    GetSwapFee {},

    // -- SQS Queries --
    #[returns(AllTicksResponse)]
    AllTicks {
        /// The tick id to start after for pagination (inclusive)
        start_from: Option<i64>,
        /// A max tick id to end at if limit is not reached/provided (inclusive)
        end_at: Option<i64>,
        /// The limit for amount of items to return
        limit: Option<usize>,
    },
    #[returns(MakerFee)]
    GetMakerFee {},

    // -- Auth Queries --
    #[returns(Option<Addr>)]
    Auth(AuthQueryMsg),

    #[returns(bool)]
    IsActive {},
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
    pub token_out: ProtoCoin,
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
pub struct GetSwapFeeResponse {
    pub swap_fee: Decimal,
}

#[cw_serde]
pub struct MakerFee {
    pub maker_fee: Decimal256,
}

#[cw_serde]
pub struct TickIdAndState {
    pub tick_id: i64,
    pub tick_state: TickState,
}

#[cw_serde]
pub struct AllTicksResponse {
    pub ticks: Vec<TickIdAndState>,
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
    // SwapToTick functions exactly as SwapExactAmountIn, but it terminates the swap when the target tick
    // is reached.
    SwapToTick {
        sender: String,
        token_in: Coin,
        token_out_denom: String,
        token_out_min_amount: Uint128,
        swap_fee: Decimal,
        target_tick: i64,
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

    // -- Active Switch
    SetActive {
        active: bool,
    },
}

#[cw_serde]
/// Fixing token in amount makes token amount out varies
pub struct SwapExactAmountInResponseData {
    pub token_out_amount: Uint256,
}

#[cw_serde]
/// Fixing token out amount makes token amount in varies
pub struct SwapExactAmountOutResponseData {
    pub token_in_amount: Uint256,
}

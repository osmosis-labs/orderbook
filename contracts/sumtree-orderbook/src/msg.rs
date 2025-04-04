use crate::types::{LimitOrder, OrderDirection, TickState};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, Uint128, Uint256};
use osmosis_std::types::cosmos::base::v1beta1::Coin as ProtoCoin;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub base_denom: String,
    pub quote_denom: String,
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
pub struct MigrateMsg {}

#[cw_serde]
pub struct DenomsResponse {
    pub quote_denom: String,
    pub base_denom: String,
}

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
    // Duplicate of the above, but for compatibility CosmWasm Pool module
    #[returns(CalcOutAmtGivenInResponse)]
    CalcOutAmtGivenIn {
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
    #[returns(TicksResponse)]
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

    // -- Contract Queries --
    #[returns(bool)]
    IsActive {},

    #[returns(OrdersResponse)]
    OrdersByOwner {
        // The address of the order maker
        owner: Addr,
        // For indexed based pagination (tick_id, order_id), exclusive
        start_from: Option<(i64, u64)>,
        // For indexed based pagination (tick_id, order_id), inclusive
        end_at: Option<(i64, u64)>,
        // Defaults to 100
        limit: Option<u64>,
    },

    #[returns(OrdersResponse)]
    OrdersByTick {
        tick_id: i64,
        start_from: Option<u64>,
        end_at: Option<u64>,
        limit: Option<u64>,
    },

    #[returns(crate::types::Orderbook)]
    OrderbookState {},

    #[returns(DenomsResponse)]
    Denoms {},

    #[returns(TicksResponse)]
    TicksById { tick_ids: Vec<i64> },

    #[returns(GetUnrealizedCancelsResponse)]
    GetUnrealizedCancels { tick_ids: Vec<i64> },
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
pub struct TicksResponse {
    pub ticks: Vec<TickIdAndState>,
}

#[cw_serde]
pub struct UnrealizedCancels {
    pub ask_unrealized_cancels: Decimal256,
    pub bid_unrealized_cancels: Decimal256,
}

#[cw_serde]
pub struct TickUnrealizedCancels {
    pub tick_id: i64,
    pub unrealized_cancels: UnrealizedCancels,
}

#[cw_serde]
pub struct GetUnrealizedCancelsResponse {
    pub ticks: Vec<TickUnrealizedCancels>,
}

#[cw_serde]
pub struct OrdersResponse {
    pub orders: Vec<LimitOrder>,
    pub count: u64,
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

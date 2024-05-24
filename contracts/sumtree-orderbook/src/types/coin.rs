use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CosmosMsg, Uint128, Uint256};
use osmosis_std::types::cosmos::bank::v1beta1::MsgSend;
use osmosis_std::types::cosmos::base::v1beta1::Coin as ProtoCoin;

/// A replication of `cosmwasmstd::coin` using `Uint256`.
pub fn coin_u256(amount: impl Into<Uint256>, denom: &str) -> Coin256 {
    Coin256 {
        amount: amount.into(),
        denom: denom.to_string(),
    }
}

/// A replication of `cosmwasmstd::Coin` using `Uint256`.
///
/// Implements conversions from `cosmwasmstd::Coin` and `osmosis_std::Coin` to help with generating bank messages.
#[cw_serde]
pub struct Coin256 {
    pub amount: Uint256,
    pub denom: String,
}

impl From<Coin256> for ProtoCoin {
    fn from(coin: Coin256) -> Self {
        ProtoCoin {
            amount: coin.amount.to_string(),
            denom: coin.denom,
        }
    }
}

impl From<Coin256> for Coin {
    fn from(coin: Coin256) -> Self {
        Coin {
            amount: Uint128::from_str(&coin.amount.to_string()).unwrap(),
            denom: coin.denom,
        }
    }
}

/// A replication of `BankMsg::Send` using `Coin256`.
///
/// Implements conversions to cosmwasm_std::CosmosMsg` and `super::MsgSend` to help with attaching these messages to `Response` objects.
#[cw_serde]
pub struct MsgSend256 {
    pub amount: Vec<Coin256>,
    pub to_address: String,
    pub from_address: String,
}

impl From<MsgSend256> for MsgSend {
    fn from(msg: MsgSend256) -> Self {
        MsgSend {
            amount: msg
                .amount
                .iter()
                .map(|c| ProtoCoin::from(c.clone()))
                .collect(),
            to_address: msg.to_address,
            from_address: msg.from_address,
        }
    }
}

impl From<MsgSend256> for CosmosMsg {
    fn from(msg: MsgSend256) -> Self {
        let msg: MsgSend = msg.into();
        msg.into()
    }
}

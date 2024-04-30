use cosmwasm_std::Uint256;
use osmosis_std::types::cosmos::base::v1beta1::Coin;

pub fn coin_u256(amount: impl Into<Uint256>, denom: &str) -> Coin {
    Coin {
        amount: amount.into().to_string(),
        denom: denom.to_string(),
    }
}

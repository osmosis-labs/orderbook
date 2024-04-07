use cosmwasm_std::Decimal256;

pub fn decimal256_from_u128(input: impl Into<u128>) -> Decimal256 {
    Decimal256::from_ratio(input.into(), 1u128)
}

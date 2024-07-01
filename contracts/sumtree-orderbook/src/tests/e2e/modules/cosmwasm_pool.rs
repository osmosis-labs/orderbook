use osmosis_std::types::osmosis::cosmwasmpool::v1beta1::{
    ContractInfoByPoolIdRequest, ContractInfoByPoolIdResponse, MsgCreateCosmWasmPool,
    MsgCreateCosmWasmPoolResponse,
};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, MsgSwapExactAmountOut,
    MsgSwapExactAmountOutResponse,
};
use osmosis_test_tube::{fn_execute, fn_query};

use osmosis_test_tube::Module;
use osmosis_test_tube::Runner;

pub struct CosmwasmPool<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for CosmwasmPool<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> CosmwasmPool<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub create_cosmwasm_pool: MsgCreateCosmWasmPool => MsgCreateCosmWasmPoolResponse
    }

    fn_execute! {
        pub swap_exact_amount_in: MsgSwapExactAmountIn => MsgSwapExactAmountInResponse
    }

    fn_execute! {
        pub swap_exact_amount_out: MsgSwapExactAmountOut => MsgSwapExactAmountOutResponse
    }

    fn_query! {
        pub contract_info_by_pool_id ["/osmosis.cosmwasmpool.v1beta1.Query/ContractInfoByPoolId"]: ContractInfoByPoolIdRequest => ContractInfoByPoolIdResponse
    }
}

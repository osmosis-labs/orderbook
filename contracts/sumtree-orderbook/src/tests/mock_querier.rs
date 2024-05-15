use cosmwasm_std::{
    coin, from_json,
    testing::{MockApi, MockQuerier, MockStorage},
    to_json_binary, BankQuery, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SupplyResponse, SystemError, SystemResult,
};

use super::test_constants::{BASE_DENOM, QUOTE_DENOM};

pub(crate) struct WasmMockQuerier {
    pub base: MockQuerier,
}

pub(crate) fn mock_dependencies_custom() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(MockQuerier::new(&[]));
    let storage = MockStorage::default();
    OwnedDeps {
        storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<cosmwasm_std::Empty> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {e}"),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&self.base, &request)
    }
}

impl WasmMockQuerier {
    pub(crate) fn handle_query(
        &self,
        querier: &MockQuerier,
        request: &QueryRequest<cosmwasm_std::Empty>,
    ) -> QuerierResult {
        match &request {
            QueryRequest::Bank(BankQuery::Supply { denom }) => match denom.as_str() {
                BASE_DENOM => {
                    let mut resp = SupplyResponse::default();
                    resp.amount = coin(1000000000000u128, denom);
                    QuerierResult::Ok(ContractResult::Ok(to_json_binary(&resp).unwrap()))
                }
                QUOTE_DENOM => {
                    let mut resp = SupplyResponse::default();
                    resp.amount = coin(1000000000000u128, denom);
                    QuerierResult::Ok(ContractResult::Ok(to_json_binary(&resp).unwrap()))
                }
                _ => querier.handle_query(request),
            },
            _ => querier.handle_query(request),
        }
    }

    fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}

use std::{collections::HashMap, path::PathBuf};

use crate::{
    msg::{DenomsResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

use cosmwasm_std::{to_json_binary, Coin};
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryAllBalancesRequest,
    cosmwasm::wasm::v1::MsgExecuteContractResponse,
    osmosis::cosmwasmpool::v1beta1::{
        ContractInfoByPoolIdRequest, ContractInfoByPoolIdResponse, MsgCreateCosmWasmPool,
    },
};
use osmosis_test_tube::{
    osmosis_std::types::osmosis::cosmwasmpool::v1beta1::UploadCosmWasmPoolCodeAndWhiteListProposal,
    GovWithAppAccess,
};

use osmosis_test_tube::{
    Account, Bank, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult, RunnerResult,
    SigningAccount, Wasm,
};
use serde::de::DeserializeOwned;

use super::modules::cosmwasm_pool::CosmwasmPool;

pub struct TestEnv<'a> {
    pub app: &'a OsmosisTestApp,
    pub creator: SigningAccount,
    pub contract: OrderbookContract<'a>,
    pub accounts: HashMap<String, SigningAccount>,
}

impl<'a> TestEnv<'a> {
    pub fn add_account(&mut self, username: &str, balance: Vec<Coin>) {
        let account = self.app.init_account(&balance).unwrap();
        self.accounts.insert(username.to_string(), account);
    }

    pub fn _assert_account_balances(
        &self,
        account: &str,
        expected_balances: Vec<Coin>,
        ignore_denoms: Vec<&str>,
    ) {
        let account_balances: Vec<Coin> = self
            ._get_account_balance(account)
            .iter()
            .filter(|coin| !ignore_denoms.contains(&coin.denom.as_str()))
            .cloned()
            .collect();

        assert_eq!(account_balances, expected_balances);
    }

    pub fn assert_contract_balances(&self, expected_balances: &[Coin], label: &str) {
        let contract_balances: Vec<Coin> = self.get_balance(&self.contract.contract_addr);

        assert_eq!(
            contract_balances, expected_balances,
            "{}: contract balances did not match",
            label
        );
    }

    pub fn get_balance(&self, address: &str) -> Vec<Coin> {
        let account_balances: Vec<Coin> = Bank::new(self.app)
            .query_all_balances(&QueryAllBalancesRequest {
                address: address.to_string(),
                pagination: None,
            })
            .unwrap()
            .balances
            .into_iter()
            .map(|coin| Coin::new(coin.amount.parse().unwrap(), coin.denom))
            .collect();

        account_balances
    }

    pub fn _get_account_balance(&self, account: &str) -> Vec<Coin> {
        let account = self.accounts.get(account).unwrap();

        self.get_balance(&account.address())
    }
}

pub struct TestEnvBuilder {
    account_balances: HashMap<String, Vec<Coin>>,
    instantiate_msg: Option<InstantiateMsg>,
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        Self {
            account_balances: HashMap::new(),
            instantiate_msg: None,
        }
    }

    pub fn with_instantiate_msg(mut self, msg: InstantiateMsg) -> Self {
        self.instantiate_msg = Some(msg);
        self
    }

    pub fn with_account(mut self, account: &str, balance: Vec<Coin>) -> Self {
        self.account_balances.insert(account.to_string(), balance);
        self
    }
    pub fn build(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts: HashMap<_, _> = self
            .account_balances
            .into_iter()
            .map(|(account, balance)| {
                let balance: Vec<_> = balance
                    .into_iter()
                    .chain(vec![Coin::new(1000000000000, "uosmo")])
                    .collect();

                (account, app.init_account(&balance).unwrap())
            })
            .collect();

        let creator = app
            .init_account(&[Coin::new(1000000000000000u128, "uosmo")])
            .unwrap();

        let instantiate_msg = self.instantiate_msg.expect("instantiate msg not set");
        let instantiate_msg = InstantiateMsg { ..instantiate_msg };

        let contract = OrderbookContract::deploy(app, &instantiate_msg, &creator).unwrap();

        TestEnv {
            app,
            creator,
            contract,
            accounts,
        }
    }
}

pub struct OrderbookContract<'a> {
    app: &'a OsmosisTestApp,
    pub code_id: u64,
    pub pool_id: u64,
    pub contract_addr: String,
}

impl<'a> OrderbookContract<'a> {
    pub fn deploy(
        app: &'a OsmosisTestApp,
        instantiate_msg: &InstantiateMsg,
        signer: &SigningAccount,
    ) -> Result<Self, RunnerError> {
        let cp = CosmwasmPool::new(app);
        let gov = GovWithAppAccess::new(app);

        let code_id = 1; // temporary solution
        gov.propose_and_execute(
            UploadCosmWasmPoolCodeAndWhiteListProposal::TYPE_URL.to_string(),
            UploadCosmWasmPoolCodeAndWhiteListProposal {
                title: String::from("store test cosmwasm pool code"),
                description: String::from("test"),
                wasm_byte_code: Self::get_wasm_byte_code(),
            },
            signer.address(),
            signer,
        )?;

        let res = cp.create_cosmwasm_pool(
            MsgCreateCosmWasmPool {
                code_id,
                instantiate_msg: to_json_binary(instantiate_msg).unwrap().to_vec(),
                sender: signer.address(),
            },
            signer,
        )?;

        let pool_id = res.data.pool_id;

        let ContractInfoByPoolIdResponse {
            contract_address,
            code_id: _,
        } = cp.contract_info_by_pool_id(&ContractInfoByPoolIdRequest { pool_id })?;

        Ok(Self {
            app,
            code_id,
            pool_id,
            contract_addr: contract_address,
        })
    }

    pub fn execute(
        &self,
        msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, msg, funds, signer)
    }

    pub fn query<Res>(&self, msg: &QueryMsg) -> RunnerResult<Res>
    where
        Res: ?Sized + DeserializeOwned,
    {
        let wasm = Wasm::new(self.app);
        wasm.query(&self.contract_addr, msg)
    }

    pub fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("target")
                .join("wasm32-unknown-unknown")
                .join("release")
                .join("sumtree_orderbook.wasm"),
        )
        .unwrap()
    }

    pub fn get_denoms(&self) -> DenomsResponse {
        self.query(&QueryMsg::Denoms {}).unwrap()
    }
}

pub fn _assert_contract_err(expected: ContractError, actual: RunnerError) {
    match actual {
        RunnerError::ExecuteError { msg } => {
            if !msg.contains(&expected.to_string()) {
                panic!(
                    "assertion failed:\n\n  must contain \t: \"{}\",\n  actual \t: \"{}\"\n",
                    expected, msg
                )
            }
        }
        _ => panic!("unexpected error, expect execute error but got: {}", actual),
    };
}
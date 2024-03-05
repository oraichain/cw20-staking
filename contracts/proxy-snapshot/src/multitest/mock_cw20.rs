use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use cw20::{BalanceResponse, Cw20Coin};
use cw20_base::{
    contract::{execute, instantiate, migrate, query},
    ContractError,
};
use oraiswap::cw_multi_test::{App, ContractWrapper, Executor};

#[cw_serde]
pub struct MockCw20Contract(Addr);

impl MockCw20Contract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
        app.store_code(Box::new(contract))
    }

    pub fn instantiate(app: &mut App, sender: &Addr, admin: &Addr) -> Result<Self, ContractError> {
        let code_id = Self::store_code(app);
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "MockCw20".to_string(),
                symbol: "MCW".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: sender.to_string(),
                    amount: Uint128::from(10000000u64),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "MockCw20",
            Some(admin.to_string()),
        )
        .map(MockCw20Contract)
        .map_err(|err| err.downcast().unwrap())
    }

    pub fn query_balance(&self, app: &App, addr: &Addr) -> BalanceResponse {
        app.wrap()
            .query_wasm_smart::<_>(
                self.0.clone(),
                &cw20_base::msg::QueryMsg::Balance {
                    address: addr.clone().to_string(),
                },
            )
            .unwrap()
    }

    pub fn transfer(&self, app: &mut App, sender: &Addr, addr: &Addr, amount: Uint128) {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &cw20_base::msg::ExecuteMsg::Transfer {
                recipient: addr.clone().into(),
                amount,
            },
            &[],
        )
        .unwrap();
    }
}

impl From<MockCw20Contract> for Addr {
    fn from(contract: MockCw20Contract) -> Addr {
        contract.0
    }
}

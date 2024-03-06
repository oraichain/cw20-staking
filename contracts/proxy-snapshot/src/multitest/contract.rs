use crate::msg::{
    ConfigTokenStakingResponse, InstantiateMsg, QueryMsg, StakedBalanceAtHeightResponse,
    TotalStakedAtHeightResponse,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_multi_test::{App, ContractWrapper, Executor};

use crate::contract::{execute, instantiate, migrate, query};

#[cw_serde]
pub struct ProxySnapshot(Addr);

impl ProxySnapshot {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
        app.store_code(Box::new(contract))
    }

    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        sender: &Addr,
        owner: &Addr,
        asset_key: &Addr,
        staking_contract: &Addr,
        admin: Option<String>,
    ) -> ProxySnapshot {
        let code_id = Self::store_code(app);
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner: Some(owner.clone()),
                asset_key: asset_key.clone(),
                staking_contract: staking_contract.clone(),
            },
            &[],
            "treasury contract",
            admin,
        )
        .map(ProxySnapshot)
        .unwrap()
    }

    #[track_caller]
    pub fn query_staked_balace_at_height(
        &self,
        app: &App,
        address: &Addr,
        height: Option<u64>,
    ) -> StakedBalanceAtHeightResponse {
        app.wrap()
            .query_wasm_smart(
                self.addr(),
                &QueryMsg::StakedBalanceAtHeight {
                    address: address.to_string(),
                    height,
                },
            )
            .unwrap()
    }

    #[track_caller]
    pub fn query_total_staked_at_height(
        &self,
        app: &App,
        height: Option<u64>,
    ) -> TotalStakedAtHeightResponse {
        app.wrap()
            .query_wasm_smart(self.addr(), &QueryMsg::TotalStakedAtHeight { height })
            .unwrap()
    }
    #[track_caller]
    pub fn query_config_token_staking(&self, app: &App) -> ConfigTokenStakingResponse {
        app.wrap()
            .query_wasm_smart(self.addr(), &QueryMsg::GetConfig {})
            .unwrap()
    }
}

impl From<ProxySnapshot> for Addr {
    fn from(contract: ProxySnapshot) -> Addr {
        contract.0
    }
}

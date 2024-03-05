use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use oraiswap::cw_multi_test::{App, ContractWrapper, Executor};
use oraiswap_staking::contract::{execute, instantiate, migrate, query};
use oraiswap_staking::msg::{
    InstantiateMsg, QueryMsg, StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
};

#[cw_serde]
pub struct Cw20Staking(Addr);

impl Cw20Staking {
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
        admin: Option<String>,
    ) -> Cw20Staking {
        let code_id = Self::store_code(app);
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner: Some(owner.clone()),
                rewarder: Addr::unchecked("rewarder"),
            },
            &[],
            "treasury contract",
            admin,
        )
        .map(Cw20Staking)
        .unwrap()
    }

    #[track_caller]
    pub fn query_staked_balace_at_height(
        &self,
        app: &App,
        address: &Addr,
        asset_key: &Addr,
        height: Option<u64>,
    ) -> StakedBalanceAtHeightResponse {
        app.wrap()
            .query_wasm_smart(
                self.addr(),
                &QueryMsg::StakedBalanceAtHeight {
                    asset_key: asset_key.clone(),
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
        asset_key: &Addr,
        height: Option<u64>,
    ) -> TotalStakedAtHeightResponse {
        app.wrap()
            .query_wasm_smart(
                self.addr(),
                &QueryMsg::TotalStakedAtHeight {
                    asset_key: asset_key.clone(),
                    height,
                },
            )
            .unwrap()
    }
}

impl From<Cw20Staking> for Addr {
    fn from(contract: Cw20Staking) -> Addr {
        contract.0
    }
}

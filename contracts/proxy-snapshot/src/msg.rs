use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Uint128};
use cw_utils::Duration;

#[cw_serde]
pub struct InstantiateMsg {
    // default is sender
    pub owner: Option<Addr>,
    pub asset_key: Addr,
    pub staking_contract: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        owner: Option<Addr>,
        asset_key: Option<Addr>,
        staking_contract: Option<Addr>,
    },
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigTokenStakingResponse)]
    GetConfig {},
    #[returns(ConfigResponse)]
    Config {},
    // snapshot
    #[returns(StakedBalanceAtHeightResponse)]
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    #[returns(TotalStakedAtHeightResponse)]
    TotalStakedAtHeight { height: Option<u64> },
}

#[cw_serde]
pub struct ConfigTokenStakingResponse {
    pub token_address: Addr,
    pub unstaking_duration: Option<Duration>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub asset_key: Addr,
    pub staking_contract: Addr,
}

#[cw_serde]
pub struct StakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct TotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

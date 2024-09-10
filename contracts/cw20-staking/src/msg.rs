use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{Asset, AssetInfo};

#[cw_serde]
pub struct InstantiateMsg {
    // default is sender
    pub owner: Option<Addr>,
    pub rewarder: Addr,
    pub withdraw_fee_receiver: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ////////////////////////
    /// Owner operations ///
    ////////////////////////
    UpdateConfig {
        rewarder: Option<Addr>,
        owner: Option<Addr>,
    },
    UpdateUnbondingPeriod {
        staking_token: Addr,
        unbonding_period: u64,
    },
    RegisterAsset {
        staking_token: Addr,
        unbonding_period: Option<u64>,
    },
    // update rewards per second for an asset
    UpdateRewardsPerSec {
        staking_token: Addr,
        assets: Vec<Asset>,
    },
    // reward tokens are in amount proportionaly, and used by minter contract to update amounts after checking the balance, which
    // will be used as rewards for the specified asset's staking pool.
    DepositReward {
        rewards: Vec<RewardMsg>,
    },

    ////////////////////////
    /// User operations ///
    ////////////////////////
    Unbond {
        staking_token: Addr,
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {
        // If the asset token is not given, then all rewards are withdrawn
        staking_token: Option<Addr>,
    },
    // Withdraw for others in this pool, such as when rewards per second are changed for the pool
    WithdrawOthers {
        staking_token: Option<Addr>,
        staker_addrs: Vec<Addr>,
    },
    Restake {
        staking_token: Addr,
    },
    UpdateInstantWithdrawOption {
        staking_token: Addr,
        period: u64,
        fee: Decimal,
    },
    RemoveInstantWithdrawOption {
        staking_token: Addr,
        period: u64,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    // this call from LP token contract
    Bond {},
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct AmountInfo {
    pub asset_info: AssetInfo,
    pub amount: Uint128,
    // pub new_staking_token: Addr,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(PoolInfoResponse)]
    PoolInfo { staking_token: Addr },
    #[returns(RewardsPerSecResponse)]
    RewardsPerSec { staking_token: Addr },
    #[returns(RewardInfoResponse)]
    RewardInfo {
        staker_addr: Addr,
        staking_token: Option<Addr>,
    },
    #[returns(Vec<RewardInfoResponse>)]
    // Query all staker belong to the pool
    RewardInfos {
        staking_token: Addr,
        start_after: Option<Addr>,
        limit: Option<u32>,
        // so can convert or throw error
        order: Option<i32>,
    },
    #[returns(Vec<QueryPoolInfoResponse>)]
    GetPoolsInformation {},
    #[returns(LockInfosResponse)]
    LockInfos {
        staker_addr: Addr,
        staking_token: Addr,
        start_after: Option<u64>,
        limit: Option<u32>,
        // so can convert or throw error
        order: Option<i32>,
    },
    // snapshot
    #[returns(StakedBalanceAtHeightResponse)]
    StakedBalanceAtHeight {
        asset_key: Addr,
        address: String,
        height: Option<u64>,
    },
    #[returns(TotalStakedAtHeightResponse)]
    TotalStakedAtHeight {
        asset_key: Addr,
        height: Option<u64>,
    },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub rewarder: Addr,
}

#[cw_serde]
pub struct RewardsPerSecResponse {
    pub assets: Vec<Asset>,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolInfoResponse {
    pub staking_token: Addr,
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
    pub unbonding_period: Option<u64>,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct RewardInfoResponse {
    pub staker_addr: Addr,
    pub reward_infos: Vec<RewardInfoResponseItem>,
}

#[cw_serde]
pub struct RewardInfoResponseItem {
    pub staking_token: Addr,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
    pub pending_withdraw: Vec<Asset>,
}

#[cw_serde]
pub struct RewardMsg {
    pub staking_token: Addr,
    pub total_accumulation_amount: Uint128,
}

#[cw_serde]
pub struct QueryPoolInfoResponse {
    pub asset_key: String,
    pub pool_info: PoolInfoResponse,
}

#[cw_serde]
pub struct LockInfo {
    pub amount: Uint128,
    pub unlock_time: Timestamp,
}

#[cw_serde]
pub struct LockInfoResponse {
    pub amount: Uint128,
    pub unlock_time: u64,
}

#[cw_serde]
pub struct LockInfosResponse {
    pub staker_addr: Addr,
    pub staking_token: Addr,
    pub lock_infos: Vec<LockInfoResponse>,
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

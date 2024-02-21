#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::rewards::{
    deposit_reward, process_reward_assets, query_all_reward_infos, query_reward_info,
    withdraw_reward, withdraw_reward_others,
};
use crate::staking::{bond, unbond};
use crate::state::{
    read_all_pool_infos, read_config, read_pool_info, read_rewards_per_sec, read_user_lock_info,
    stakers_read, store_config, store_pool_info, store_rewards_per_sec, store_unbonding_period,
    Config, PoolInfo,
};

use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockInfoResponse, LockInfosResponse,
    MigrateMsg, PoolInfoResponse, QueryMsg, QueryPoolInfoResponse, RewardsPerSecResponse,
};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Api, Binary, CanonicalAddr, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdError, StdResult, Uint128,
};
use oraiswap::asset::{Asset, AssetRaw};

use cw20::Cw20ReceiveMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps
                .api
                .addr_canonicalize(msg.owner.unwrap_or(info.sender.clone()).as_str())?,
            rewarder: deps.api.addr_canonicalize(msg.rewarder.as_str())?,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::UpdateConfig { rewarder, owner } => update_config(deps, info, owner, rewarder),
        ExecuteMsg::UpdateRewardsPerSec {
            staking_token,
            assets,
        } => update_rewards_per_sec(deps, info, staking_token, assets),
        ExecuteMsg::DepositReward { rewards } => deposit_reward(deps, info, rewards),
        ExecuteMsg::RegisterAsset {
            staking_token,
            unbonding_period,
        } => register_asset(deps, info, staking_token, unbonding_period),
        ExecuteMsg::Unbond {
            staking_token,
            amount,
        } => unbond(deps, env, info.sender, staking_token, amount),
        ExecuteMsg::Withdraw { staking_token } => withdraw_reward(deps, env, info, staking_token),
        ExecuteMsg::WithdrawOthers {
            staking_token,
            staker_addrs,
        } => withdraw_reward_others(deps, env, info, staker_addrs, staking_token),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Bond {}) => bond(
            deps,
            Addr::unchecked(cw20_msg.sender),
            info.sender,
            cw20_msg.amount,
        ),
        Err(_) => Err(StdError::generic_err("invalid cw20 hook message")),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    rewarder: Option<Addr>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(owner.as_str())?;
    }

    if let Some(rewarder) = rewarder {
        config.rewarder = deps.api.addr_canonicalize(rewarder.as_str())?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

// need to withdraw all rewards of the stakers belong to the pool
// may need to call withdraw from backend side by querying all stakers with pagination in case out of gas
fn update_rewards_per_sec(
    deps: DepsMut,
    info: MessageInfo,
    staking_token: Addr,
    assets: Vec<Asset>,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = deps.api.addr_canonicalize(staking_token.as_str())?.to_vec();

    // withdraw all rewards for all stakers from this pool
    let staker_addrs = stakers_read(deps.storage, &asset_key)
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (k, _) = item?;
            Ok(CanonicalAddr::from(k))
        })
        .collect::<StdResult<Vec<CanonicalAddr>>>()?;

    // let mut messages: Vec<CosmosMsg> = vec![];

    // withdraw reward for each staker
    for staker_addr_raw in staker_addrs {
        process_reward_assets(
            deps.storage,
            &staker_addr_raw,
            &Some(asset_key.clone()),
            false,
        )?;
    }

    // convert assets to raw_assets
    let raw_assets = assets
        .into_iter()
        .map(|w| Ok(w.to_raw(deps.api)?))
        .collect::<StdResult<Vec<AssetRaw>>>()?;

    store_rewards_per_sec(deps.storage, &asset_key, raw_assets)?;

    Ok(Response::new().add_attribute("action", "update_rewards_per_sec"))
}

fn register_asset(
    deps: DepsMut,
    info: MessageInfo,
    staking_token: Addr,
    unbonding_period: Option<u64>,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    if config.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    // query asset_key from AssetInfo
    let asset_key = deps.api.addr_canonicalize(staking_token.as_str())?;
    if read_pool_info(deps.storage, &asset_key).is_ok() {
        return Err(StdError::generic_err("Asset was already registered"));
    }

    store_pool_info(
        deps.storage,
        &asset_key.clone(),
        &PoolInfo {
            staking_token: asset_key,
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        },
    )?;

    if let Some(unbonding_period) = unbonding_period {
        if unbonding_period > 0 {
            store_unbonding_period(deps.storage, staking_token.as_bytes(), unbonding_period)?;
        }
    }

    Ok(Response::new().add_attributes([
        ("action", "register_asset"),
        ("staking_token", staking_token.as_str()),
        (
            "unbonding_period",
            &unbonding_period.unwrap_or(0).to_string(),
        ),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::PoolInfo { staking_token } => to_binary(&query_pool_info(deps, staking_token)?),
        QueryMsg::RewardsPerSec { staking_token } => {
            to_binary(&query_rewards_per_sec(deps, staking_token)?)
        }
        QueryMsg::RewardInfo {
            staker_addr,
            staking_token,
        } => to_binary(&query_reward_info(deps, staker_addr, staking_token)?),
        QueryMsg::RewardInfos {
            staking_token,
            start_after,
            limit,
            order,
        } => to_binary(&query_all_reward_infos(
            deps,
            staking_token,
            start_after,
            limit,
            order,
        )?),
        QueryMsg::GetPoolsInformation {} => to_binary(&query_get_pools_infomation(deps)?),
        QueryMsg::LockInfos {
            staker_addr,
            staking_token,
            start_after,
            limit,
            order,
        } => to_binary(&query_lock_infos(
            deps,
            _env,
            staker_addr,
            staking_token,
            start_after,
            limit,
            order,
        )?),
    }
}

pub fn query_lock_infos(
    deps: Deps,
    _env: Env,
    staker_addr: Addr,
    staking_token: Addr,
    start_after: Option<u64>,
    limit: Option<u32>,
    order: Option<i32>,
) -> StdResult<LockInfosResponse> {
    let lock_infos = read_user_lock_info(
        deps.storage,
        staking_token.as_bytes(),
        staker_addr.as_bytes(),
        start_after,
        limit,
        order,
    )?;
    Ok(LockInfosResponse {
        staker_addr,
        staking_token,
        lock_infos: lock_infos
            .into_iter()
            .map(|lock| LockInfoResponse {
                amount: lock.amount,
                unlock_time: lock.unlock_time.seconds(),
            })
            .collect(),
    })
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?,
        rewarder: deps.api.addr_humanize(&state.rewarder)?,
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, staking_token: Addr) -> StdResult<PoolInfoResponse> {
    let asset_key = deps.api.addr_canonicalize(staking_token.as_str())?;
    let pool_info = read_pool_info(deps.storage, &asset_key)?;
    Ok(PoolInfoResponse {
        staking_token: deps.api.addr_humanize(&pool_info.staking_token)?,
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
    })
}

pub fn query_rewards_per_sec(deps: Deps, staking_token: Addr) -> StdResult<RewardsPerSecResponse> {
    let asset_key = deps.api.addr_canonicalize(staking_token.as_str())?.to_vec();

    let raw_assets = read_rewards_per_sec(deps.storage, &asset_key)?;

    let assets = raw_assets
        .into_iter()
        .map(|w| Ok(w.to_normal(deps.api)?))
        .collect::<StdResult<Vec<Asset>>>()?;

    Ok(RewardsPerSecResponse { assets })
}

pub fn parse_read_all_pool_infos(
    api: &dyn Api,
    pool_infos: Vec<(Vec<u8>, PoolInfo)>,
) -> StdResult<Vec<QueryPoolInfoResponse>> {
    pool_infos
        .into_iter()
        .map(|(key, pool_info)| {
            let asset_key = CanonicalAddr::from(key);
            let staking_token = api.addr_humanize(&asset_key)?;
            Ok(QueryPoolInfoResponse {
                asset_key: staking_token.to_string(),
                pool_info: PoolInfoResponse {
                    staking_token,
                    total_bond_amount: pool_info.total_bond_amount,
                    reward_index: pool_info.reward_index,
                    pending_reward: pool_info.pending_reward,
                },
            })
        })
        .collect::<StdResult<Vec<QueryPoolInfoResponse>>>()
}

pub fn query_get_pools_infomation(deps: Deps) -> StdResult<Vec<QueryPoolInfoResponse>> {
    let pool_infos = read_all_pool_infos(deps.storage)?;
    parse_read_all_pool_infos(deps.api, pool_infos)
}

// migrate contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

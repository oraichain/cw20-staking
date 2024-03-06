use crate::{
    msg::{
        ConfigTokenStakingResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
        StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
    },
    state::{Config, CONFIG},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw_utils::Duration;
use oraiswap_staking::msg::PoolInfoResponse;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let owner = msg.owner.unwrap_or(info.sender);
    let config = Config {
        owner: deps.api.addr_validate(owner.as_ref())?,
        asset_key: deps.api.addr_validate(msg.asset_key.as_ref())?,
        staking_contract: deps.api.addr_validate(msg.staking_contract.as_ref())?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            asset_key,
            staking_contract,
        } => update_config(deps, env, info, owner, asset_key, staking_contract),
    }
}

fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<Addr>,
    asset_key: Option<Addr>,
    staking_contract: Option<Addr>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let new_config = Config {
        owner: owner.unwrap_or(config.owner),
        asset_key: asset_key.unwrap_or(config.asset_key),
        staking_contract: staking_contract.unwrap_or(config.staking_contract),
    };
    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("owner", new_config.owner.as_ref())
        .add_attribute("asset_key", new_config.asset_key.as_ref())
        .add_attribute("staking_contract", new_config.staking_contract.as_ref()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::TotalStakedAtHeight { height } => {
            to_binary(&query_total_staked_at_height(deps, env, height)?)
        }
        QueryMsg::StakedBalanceAtHeight { height, address } => {
            to_binary(&query_staked_balance_at_height(deps, env, address, height)?)
        }
        QueryMsg::GetConfig {} => to_binary(&query_config_token_staking(deps, env)?),
    }
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    _env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<StakedBalanceAtHeightResponse> {
    let config = CONFIG.load(deps.storage)?;
    deps.querier
        .query_wasm_smart::<StakedBalanceAtHeightResponse>(
            config.staking_contract,
            &oraiswap_staking::msg::QueryMsg::StakedBalanceAtHeight {
                asset_key: config.asset_key,
                address,
                height,
            },
        )
}

pub fn query_total_staked_at_height(
    deps: Deps,
    _env: Env,
    height: Option<u64>,
) -> StdResult<TotalStakedAtHeightResponse> {
    let config = CONFIG.load(deps.storage)?;
    deps.querier
        .query_wasm_smart::<TotalStakedAtHeightResponse>(
            config.staking_contract,
            &oraiswap_staking::msg::QueryMsg::TotalStakedAtHeight {
                asset_key: config.asset_key,
                height,
            },
        )
}

pub fn query_config_token_staking(deps: Deps, _env: Env) -> StdResult<ConfigTokenStakingResponse> {
    let config = CONFIG.load(deps.storage)?;

    let pool_info = deps.querier.query_wasm_smart::<PoolInfoResponse>(
        config.staking_contract,
        &oraiswap_staking::msg::QueryMsg::PoolInfo {
            staking_token: config.asset_key.clone(),
        },
    )?;
    Ok(ConfigTokenStakingResponse {
        token_address: config.asset_key,
        unstaking_duration: pool_info.unbonding_period.map(Duration::Time),
    })
}

// migrate contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

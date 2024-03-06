use cosmwasm_std::{to_binary, Addr, Uint128};
use cw_utils::Duration;
use oraiswap::{
    asset::Asset,
    cw_multi_test::{App, Executor},
};
use oraiswap_staking::msg::PoolInfoResponse;

use super::{
    contract::ProxySnapshot, cw20_staking_contract::Cw20Staking, mock_cw20::MockCw20Contract,
};

#[test]
fn test_query_snapshot_balance() {
    // Arrange
    let mut app = App::default();
    let owner = Addr::unchecked("owner");

    let cw20 = MockCw20Contract::instantiate(&mut app, &owner, &owner).unwrap();
    let asset_key = cw20.addr().clone();
    // contracts instantiation
    let cw20_staking = Cw20Staking::instantiate(&mut app, &owner, &owner, Some("owner".into()));
    let snapshot = ProxySnapshot::instantiate(
        &mut app,
        &owner,
        &owner,
        &asset_key,
        cw20_staking.addr(),
        Some("owner".into()),
    );

    // setup Cw20Staking contract
    app.execute_contract(
        owner.clone(),
        cw20_staking.addr().clone(),
        &oraiswap_staking::msg::ExecuteMsg::RegisterAsset {
            staking_token: asset_key.clone(),
            unbonding_period: Some(100),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        owner.clone(),
        cw20_staking.addr().clone(),
        &oraiswap_staking::msg::ExecuteMsg::UpdateRewardsPerSec {
            staking_token: asset_key.clone(),
            assets: vec![Asset {
                info: oraiswap::asset::AssetInfo::Token {
                    contract_addr: asset_key.clone(),
                },
                amount: 100u128.into(),
            }],
        },
        &[],
    )
    .unwrap();

    // Action
    // Staked
    app.execute_contract(
        owner.clone(),
        cw20.addr().clone(),
        &cw20_base::msg::ExecuteMsg::Send {
            contract: cw20_staking.addr().clone().to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&oraiswap_staking::msg::Cw20HookMsg::Bond {}).unwrap(),
        },
        &[],
    )
    .unwrap();
    let mut current_block = app.block_info();
    // Unstaked
    app.execute_contract(
        owner.clone(),
        cw20_staking.addr().clone(),
        &oraiswap_staking::msg::ExecuteMsg::Unbond {
            staking_token: asset_key.clone(),
            amount: Uint128::from(50u128),
        },
        &[],
    )
    .unwrap();

    // increase block height to confirmed, and update snapshot
    current_block.height += 1;
    app.set_block(current_block);
    let total = snapshot.query_total_staked_at_height(&app, None);
    let staked_balance = snapshot.query_staked_balace_at_height(&app, &owner, None);

    // Assert
    let config_token_response = snapshot.query_config_token_staking(&app);

    assert_eq!(
        config_token_response.unstaking_duration,
        Some(100).map(Duration::Time)
    );
    assert_eq!(config_token_response.token_address, cw20.addr().clone());
    assert_eq!(total.total.u128(), 50u128);
    assert_eq!(staked_balance.balance.u128(), 50u128);
}

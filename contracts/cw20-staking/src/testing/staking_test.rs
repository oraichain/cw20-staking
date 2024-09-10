use crate::contract::{execute, instantiate, query, query_get_pools_infomation};
use crate::msg::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockInfosResponse, PoolInfoResponse, QueryMsg,
    RewardInfoResponse, RewardInfoResponseItem, RewardMsg, StakedBalanceAtHeightResponse,
    TotalStakedAtHeightResponse,
};
use crate::state::{store_pool_info, PoolInfo, MAX_LIMIT};
use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier,
    MockStorage,
};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, Api, BankMsg, CosmosMsg, Decimal, OwnedDeps,
    StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::testing::ATOM_DENOM;

#[test]
fn test_query_all_pool_keys() {
    let mut deps = mock_dependencies();
    let first_staking_token = Addr::unchecked("staking1");
    let second_staking_token = Addr::unchecked("staking2");
    let first_staking_canon = deps
        .api
        .addr_canonicalize(first_staking_token.as_str())
        .unwrap();
    let second_staking_canon = deps
        .api
        .addr_canonicalize(second_staking_token.as_str())
        .unwrap();

    let deps_mut = deps.as_mut();
    let storage = deps_mut.storage;

    // populate fake data, can change to 100 if want
    for n in 0..10u64 {
        let amount = Uint128::from(n);
        let (asset_key, staking_token) = if n < 5 {
            (first_staking_canon.clone(), first_staking_canon.clone())
        } else {
            (second_staking_canon.clone(), second_staking_canon.clone())
        };
        let pool_info = PoolInfo {
            staking_token: staking_token.clone(),
            pending_reward: amount.clone(),
            total_bond_amount: amount.clone(),
            reward_index: Decimal::zero(),
        };
        store_pool_info(storage, &asset_key, &pool_info).unwrap();
    }

    let all_pool_keys = query_get_pools_infomation(deps.as_ref()).unwrap();
    assert_eq!(all_pool_keys.len(), 2);
    // assert_eq!(
    //     all_pool_keys.contains(&first_staking_token.to_string()),
    //     true
    // );
    // assert_eq!(
    //     all_pool_keys.contains(&second_staking_token.to_string()),
    //     true
    // );
}

#[test]
fn test_bond_tokens() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        withdraw_fee_receiver: Addr::unchecked("withdraw_fee_receiver"),
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        staking_token: Addr::unchecked("staking"),
        unbonding_period: None,
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });

    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            staking_token: Some(Addr::unchecked("staking")),
            staker_addr: Addr::unchecked("addr"),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: Addr::unchecked("addr"),
            reward_infos: vec![RewardInfoResponseItem {
                staking_token: Addr::unchecked("staking"),
                pending_reward: Uint128::zero(),
                pending_withdraw: vec![],
                bond_amount: Uint128::from(100u128),
            }],
        }
    );

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            staking_token: Addr::unchecked("staking"),
        },
    )
    .unwrap();

    let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            staking_token: Addr::unchecked("staking"),
            total_bond_amount: Uint128::from(100u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            unbonding_period: None,
        }
    );

    // bond 100 more tokens from other account
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr2".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            staking_token: Addr::unchecked("staking"),
        },
    )
    .unwrap();
    let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            staking_token: Addr::unchecked("staking"),
            total_bond_amount: Uint128::from(200u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            unbonding_period: None
        }
    );
}

#[test]
fn test_unbond() {
    let mut deps = _setup_staking(None);

    // unbond 150 tokens; failed
    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(150u128),
    };

    let info = mock_info("addr", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot unbond more than bond amount");
        }
        _ => panic!("Must return generic error"),
    };

    // normal unbond
    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(100u128),
    };

    let info = mock_info("addr", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr".to_string(),
                amount: vec![coin(99u128, ORAI_DENOM)],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr".to_string(),
                amount: vec![coin(199u128, ATOM_DENOM)],
            })),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: "staking".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr".to_string(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }),
        ]
    );

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            staking_token: Addr::unchecked("staking"),
        },
    )
    .unwrap();
    let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            staking_token: Addr::unchecked("staking"),
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::from_ratio(300u128, 100u128),
            pending_reward: Uint128::zero(),
            unbonding_period: None
        }
    );

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            staking_token: None,
            staker_addr: Addr::unchecked("addr"),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: Addr::unchecked("addr"),
            reward_infos: vec![RewardInfoResponseItem {
                staking_token: Addr::unchecked("staking"),
                bond_amount: Uint128::from(0u128),
                pending_reward: Uint128::from(0u128),
                pending_withdraw: vec![],
            }],
        }
    );
}

#[test]
fn test_unbonding_period_happy_case() {
    let unbonding_period = 100;
    let mut deps = _setup_staking(Some(unbonding_period));

    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(50u128),
    };
    let info = mock_info("addr", &[]);
    let mut unbond_env = mock_env();

    let _res = execute(deps.as_mut(), unbond_env.clone(), info.clone(), msg).unwrap();

    assert_eq!(
        _res.attributes,
        vec![
            attr("action", "unbonding"),
            attr("staker_addr", "addr"),
            attr("amount", Uint128::from(50u128).to_string()),
            attr("staking_token", "staking"),
            attr(
                "unlock_time",
                unbond_env
                    .clone()
                    .block
                    .time
                    .plus_seconds(unbonding_period)
                    .seconds()
                    .to_string()
            ),
        ]
    );

    let res = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::LockInfos {
            staker_addr: Addr::unchecked("addr"),
            staking_token: Addr::unchecked("staking"),
            start_after: None,
            limit: None,
            order: None,
        },
    )
    .unwrap();
    let lock_ids = from_binary::<LockInfosResponse>(&res).unwrap();

    assert_eq!(lock_ids.lock_infos.len(), 1);
    assert_eq!(lock_ids.lock_infos[0].amount, Uint128::from(50u128));
    assert_eq!(
        lock_ids.lock_infos[0].unlock_time,
        unbond_env
            .clone()
            .block
            .time
            .plus_seconds(unbonding_period)
            .seconds()
    );
    assert_eq!(lock_ids.staking_token, Addr::unchecked("staking"));
    assert_eq!(lock_ids.staker_addr, Addr::unchecked("addr"));

    // increase block.time
    unbond_env.block.time = unbond_env.block.time.plus_seconds(unbonding_period + 1);
    // Unbond and withdraw_lock
    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(50u128),
    };
    let mut _res = execute(deps.as_mut(), unbond_env.clone(), info.clone(), msg).unwrap();
    _res.attributes.sort_by(|a, b| a.key.cmp(&b.key));
    let res = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::LockInfos {
            staker_addr: Addr::unchecked("addr"),
            staking_token: Addr::unchecked("staking"),
            start_after: None,
            limit: None,
            order: None,
        },
    )
    .unwrap();
    let lock_ids = from_binary::<LockInfosResponse>(&res).unwrap();

    assert_eq!(lock_ids.staking_token, Addr::unchecked("staking"));
    assert_eq!(lock_ids.staker_addr, Addr::unchecked("addr"));
    assert_eq!(
        _res.attributes,
        vec![
            attr("action", "unbond"),
            attr("action", "unbonding"),
            attr("amount", Uint128::from(50u128).to_string()),
            attr("amount", Uint128::from(50u128).to_string()),
            attr("staker_addr", "addr"),
            attr("staker_addr", "addr"),
            attr("staking_token", "staking"),
            attr("staking_token", "staking"),
            attr(
                "unlock_time",
                unbond_env
                    .clone()
                    .block
                    .time
                    .plus_seconds(unbonding_period)
                    .seconds()
                    .to_string()
            ),
        ]
    );
    assert_eq!(
        _res.messages,
        vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: "staking".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr".to_string(),
                    amount: Uint128::from(50u128),
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr".to_string(),
                amount: vec![coin(99u128, ORAI_DENOM)],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr".to_string(),
                amount: vec![coin(199u128, ATOM_DENOM)],
            })),
        ]
    );

    unbond_env.block.time = unbond_env.block.time.plus_seconds(unbonding_period + 1);

    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(0u128),
    };
    let _res = execute(deps.as_mut(), unbond_env.clone(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::LockInfos {
            staker_addr: Addr::unchecked("addr"),
            staking_token: Addr::unchecked("staking"),
            start_after: None,
            limit: None,
            order: None,
        },
    )
    .unwrap();

    let lock_ids = from_binary::<LockInfosResponse>(&res).unwrap();
    assert_eq!(lock_ids.lock_infos.len(), 0);

    assert_eq!(
        _res.attributes,
        vec![
            attr("action", "unbond"),
            attr("staker_addr", "addr"),
            attr("amount", Uint128::from(50u128).to_string()),
            attr("staking_token", "staking"),
        ]
    );
    assert_eq!(
        _res.messages,
        vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: "staking".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr".to_string(),
                amount: Uint128::from(50u128),
            })
            .unwrap(),
            funds: vec![],
        }),]
    )
}

#[test]
pub fn test_multiple_lock() {
    let unbonding_period = 10000;
    let mut deps = _setup_staking(Some(unbonding_period));
    let info = mock_info("addr", &[]);
    let mut unbond_env = mock_env();

    for i in 0..MAX_LIMIT {
        let msg = ExecuteMsg::Unbond {
            staking_token: Addr::unchecked("staking"),
            amount: Uint128::from(1u128),
        };
        let mut clone_unbonded = unbond_env.clone();
        clone_unbonded.block.time = clone_unbonded
            .block
            .time
            .plus_seconds((i as u64) * unbonding_period / 50);
        let _res = execute(deps.as_mut(), clone_unbonded, info.clone(), msg).unwrap();
    }
    let binary_response = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::LockInfos {
            staker_addr: Addr::unchecked("addr"),
            staking_token: Addr::unchecked("staking"),
            start_after: None,
            limit: Some(30),
            order: None,
        },
    )
    .unwrap();
    let lock_infos = from_binary::<LockInfosResponse>(&binary_response).unwrap();
    assert_eq!(lock_infos.lock_infos.len(), MAX_LIMIT as usize);

    // Since we anchor the timestamp by unbond_env, so we must add the unbonding_period to the
    // block_time to get the first unlock timestamp. Then, we plus another unbonding_period to get to the rest
    // of lock
    unbond_env.block.time = unbond_env.block.time.plus_seconds(unbonding_period);
    unbond_env.block.time = unbond_env.block.time.plus_seconds(unbonding_period);

    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(0u128),
    };

    let res = execute(deps.as_mut(), unbond_env.clone(), info.clone(), msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unbond"),
            attr("staker_addr", "addr"),
            attr("amount", Uint128::from(MAX_LIMIT as u128).to_string()),
            attr("staking_token", "staking"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: "staking".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr".to_string(),
                amount: Uint128::from(MAX_LIMIT as u128),
            })
            .unwrap(),
            funds: vec![],
        }),]
    );

    // assert after we withdraw all_lock
    let binary_response = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::LockInfos {
            staker_addr: Addr::unchecked("addr"),
            staking_token: Addr::unchecked("staking"),
            start_after: None,
            limit: None,
            order: None,
        },
    )
    .unwrap();
    let lock_infos = from_binary::<LockInfosResponse>(&binary_response).unwrap();
    assert_eq!(lock_infos.lock_infos.len(), 0);
}

#[test]
fn test_balance_and_bonded_snapshot() {
    let mut deps = _setup_staking(None);
    let mock_env = mock_env();
    let info = mock_info("staking", &[]);

    // staker balance
    let res = query(
        deps.as_ref(),
        mock_env.clone(),
        QueryMsg::StakedBalanceAtHeight {
            asset_key: Addr::unchecked("staking"),
            address: "sender".into(),
            height: None,
        },
    )
    .unwrap();

    let balance = from_binary::<StakedBalanceAtHeightResponse>(&res).unwrap();
    assert_eq!(balance.balance, Uint128::zero());
    assert_eq!(balance.height, mock_env.block.height);

    // total_snapshot
    let res = query(
        deps.as_ref(),
        mock_env.clone(),
        QueryMsg::TotalStakedAtHeight {
            asset_key: Addr::unchecked("staking"),
            height: Some(mock_env.clone().block.height + 1),
        },
    )
    .unwrap();

    let balance = from_binary::<TotalStakedAtHeightResponse>(&res).unwrap();
    assert_eq!(balance.total, Uint128::from(100u128));
    assert_eq!(balance.height, mock_env.block.height + 1);

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });

    let mut skip_100_blocks_env = mock_env.clone();
    skip_100_blocks_env.block.height += 100;
    let _res = execute(deps.as_mut(), skip_100_blocks_env.clone(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env.clone(),
        QueryMsg::StakedBalanceAtHeight {
            asset_key: Addr::unchecked("staking"),
            address: "sender".into(),
            height: Some(skip_100_blocks_env.block.height + 1),
        },
    )
    .unwrap();
    let balance = from_binary::<StakedBalanceAtHeightResponse>(&res).unwrap();
    assert_eq!(balance.balance, Uint128::from(100u128));
    // because the block height snapshot is after the execute block
    assert_eq!(balance.height, mock_env.block.height + 101);

    // total_snapshot
    let res = query(
        deps.as_ref(),
        mock_env.clone(),
        QueryMsg::TotalStakedAtHeight {
            asset_key: Addr::unchecked("staking"),
            height: Some(skip_100_blocks_env.block.height + 1),
        },
    )
    .unwrap();

    let balance = from_binary::<TotalStakedAtHeightResponse>(&res).unwrap();
    // because the _setup_staking already bond 100 tokens
    assert_eq!(balance.total, Uint128::from(200u8));
    assert_eq!(balance.height, mock_env.block.height + 101);

    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(100u128),
    };

    let mut skip_200_blocks_env = mock_env.clone();
    skip_200_blocks_env.block.height += 200;

    let sender_info = mock_info("sender", &[]);
    let _res = execute(deps.as_mut(), skip_200_blocks_env.clone(), sender_info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env.clone(),
        QueryMsg::StakedBalanceAtHeight {
            asset_key: Addr::unchecked("staking"),
            address: "sender".into(),
            height: Some(skip_200_blocks_env.block.height + 1),
        },
    )
    .unwrap();
    let balance = from_binary::<StakedBalanceAtHeightResponse>(&res).unwrap();
    assert_eq!(balance.balance, Uint128::zero());
    // because the block height snapshot is after the execute block
    assert_eq!(balance.height, mock_env.block.height + 201);

    // total_snapshot
    let res = query(
        deps.as_ref(),
        mock_env.clone(),
        QueryMsg::TotalStakedAtHeight {
            asset_key: Addr::unchecked("staking"),
            height: Some(skip_200_blocks_env.block.height + 1),
        },
    )
    .unwrap();

    let balance = from_binary::<TotalStakedAtHeightResponse>(&res).unwrap();
    // because the _setup_staking already bond 100 tokens
    assert_eq!(balance.total, Uint128::from(100u8));
    assert_eq!(balance.height, mock_env.block.height + 201);
}

#[test]
fn test_restake() {
    // Arrange
    let unbonding_period = 10000;
    let mut deps = _setup_staking(Some(unbonding_period));
    let info = mock_info("addr", &[]);
    let unbond_env = mock_env();

    for i in 0..MAX_LIMIT {
        let msg = ExecuteMsg::Unbond {
            staking_token: Addr::unchecked("staking"),
            amount: Uint128::from(1u128),
        };
        let mut clone_unbonded = unbond_env.clone();
        clone_unbonded.block.time = clone_unbonded
            .block
            .time
            .plus_seconds((i as u64) * unbonding_period / 50);
        let _res = execute(deps.as_mut(), clone_unbonded, info.clone(), msg).unwrap();
    }
    let binary_response = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::LockInfos {
            staker_addr: Addr::unchecked("addr"),
            staking_token: Addr::unchecked("staking"),
            start_after: None,
            limit: Some(30),
            order: None,
        },
    )
    .unwrap();
    let lock_infos = from_binary::<LockInfosResponse>(&binary_response).unwrap();
    assert_eq!(lock_infos.lock_infos.len(), MAX_LIMIT as usize);
    let pool_info_binary = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::PoolInfo {
            staking_token: Addr::unchecked("staking"),
        },
    );
    let pool_info = from_binary::<PoolInfoResponse>(&pool_info_binary.unwrap()).unwrap();
    assert_eq!(
        pool_info.total_bond_amount,
        Uint128::from(100u128 - u128::from(MAX_LIMIT))
    );

    // Act
    let msg = ExecuteMsg::Restake {
        staking_token: Addr::unchecked("staking"),
    };
    let _res = execute(deps.as_mut(), unbond_env.clone(), info, msg).unwrap();

    // Assert
    let pool_info_binary = query(
        deps.as_ref(),
        unbond_env.clone(),
        QueryMsg::PoolInfo {
            staking_token: Addr::unchecked("staking"),
        },
    );
    let pool_info = from_binary::<PoolInfoResponse>(&pool_info_binary.unwrap()).unwrap();
    assert_eq!(pool_info.total_bond_amount, Uint128::from(100u128));
}

fn _setup_staking(unbonding_period: Option<u64>) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);
    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        withdraw_fee_receiver: Addr::unchecked("withdraw_fee_receiver"),
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
        ],
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // register asset
    let msg = ExecuteMsg::RegisterAsset {
        staking_token: Addr::unchecked("staking"),
        unbonding_period,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "register_asset"),
            attr("staking_token", "staking"),
            attr(
                "unbonding_period",
                unbonding_period.unwrap_or(0).to_string()
            )
        ]
    );
    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("staking"),
            total_accumulation_amount: Uint128::from(300u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    deps
}

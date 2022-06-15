use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};
use nstable_stakepooling_v2::{HRSimpleStakePoolTerms};

use crate::common::utils::*;
use crate::common::init::deploy_stakepooling;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_create_simple_stakepool() {
    generate_user_account!(root, owner, staker1);
    
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![]);

    let (stakepooling, _) = prepair_multi_stakepools(&root, &owner, &token1, to_yocto("10"), 31);

    assert_eq!(show_stakepools_by_locktoken(&stakepooling, format!("{}@0", swap()), false).len(), 31);

    assert_err!(call!(
        staker1,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ), "ERR_NOT_ALLOWED");

    assert_err!(call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0@3", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ), "E33: invalid locktoken id");

    call!(
        owner,
        stakepooling.extend_operators(vec![staker1.valid_account_id()]),
        deposit = 1
    ).assert_success();

    call!(
        staker1,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_eq!(show_stakepools_by_locktoken(&stakepooling, format!("{}@0", swap()), false).len(), 32);

    assert_err!(call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ), "E36: the number of stakepools has reached its limit");
}


#[test]
fn test_force_clean_stakepool() {
    generate_user_account!(root, owner, staker1);
    
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1]);

    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());

    let stakepool_id = format!("{}@0#0", pool.account_id());
    let locktoken_id = format!("{}@0", pool.account_id());
    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_err!(call!(
        staker1,
        stakepooling.force_clean_stakepool(stakepool_id.clone())
    ), "ERR_NOT_ALLOWED");

    assert_err!(call!(
        owner,
        stakepooling.force_clean_stakepool("random".to_string())
    ), "E41: stakepool not exist");

    assert_err!(call!(
        owner,
        stakepooling.force_clean_stakepool(stakepool_id.clone())
    ), "StakePool can NOT be removed now");

    //Fast forward to DEFAULT_STAKEPOOL_EXPIRE_SEC without any reward
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);
    assert_err!(call!(
        owner,
        stakepooling.force_clean_stakepool(format!("{}@0#0", pool.account_id()))
    ), "StakePool can NOT be removed now");

    //add reward
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id.clone())),
        deposit = 1
    )
    .assert_success();
    
    //The rewards have been handed out, but stakepool not expire
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60 * 11);
    assert_err!(call!(
        owner,
        stakepooling.force_clean_stakepool(stakepool_id.clone())
    ), "StakePool can NOT be removed now");

    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);

    assert_eq!(show_stakepools_by_locktoken(&stakepooling, locktoken_id.clone(), false).len(), 1);
    assert_eq!(show_outdated_stakepools(&stakepooling, false).len(), 0);
    call!(
        owner,
        stakepooling.extend_operators(vec![staker1.valid_account_id()]),
        deposit = 1
    ).assert_success();
    call!(
        owner,
        stakepooling.force_clean_stakepool(stakepool_id.clone())
    ).assert_success();
    assert_eq!(show_stakepools_by_locktoken(&stakepooling, locktoken_id.clone(), false).len(), 0);
    assert_eq!(show_outdated_stakepools(&stakepooling, false).len(), 1);
}


#[test]
fn test_cancel_stakepool() {
    generate_user_account!(root, owner, staker1);
    
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1]);

    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());

    let stakepool_id = format!("{}@0#0", pool.account_id());
    let locktoken_id = format!("{}@0", pool.account_id());
    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_err!(call!(
        staker1,
        stakepooling.cancel_stakepool(stakepool_id.clone())
    ), "ERR_NOT_ALLOWED");

    assert_err!(call!(
        owner,
        stakepooling.cancel_stakepool("random".to_string())
    ), "E41: stakepool not exist");

    //add reward
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id.clone())),
        deposit = 1
    )
    .assert_success();

    //The rewards have been handed out, but stakepool not expire
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60 * 11);
    assert_err!(call!(
        owner,
        stakepooling.cancel_stakepool(stakepool_id.clone())
    ), "This stakepool can NOT be cancelled");

    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);
    assert_err!(call!(
        owner,
        stakepooling.cancel_stakepool(stakepool_id.clone())
    ), "This stakepool can NOT be cancelled");

    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_eq!(show_stakepools_by_locktoken(&stakepooling, locktoken_id.clone(), false).len(), 2);
    assert_eq!(show_outdated_stakepools(&stakepooling, false).len(), 0);
    call!(
        owner,
        stakepooling.extend_operators(vec![staker1.valid_account_id()]),
        deposit = 1
    ).assert_success();
    call!(
        staker1,
        stakepooling.cancel_stakepool(format!("{}@0#1", pool.account_id()))
    ).assert_success();
    assert_eq!(show_stakepools_by_locktoken(&stakepooling, locktoken_id.clone(), false).len(), 1);
    assert_eq!(show_outdated_stakepools(&stakepooling, false).len(), 0);
}


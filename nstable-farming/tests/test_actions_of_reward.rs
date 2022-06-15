use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};
use nstable_stakepooling_v2::{HRSimpleStakePoolTerms};

use crate::common::utils::*;
use crate::common::init::deploy_stakepooling;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_remove_user_rps_by_stakepool(){
    generate_user_account!(root, owner, staker1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1]);

    let stakepool_id = "swap@0#0".to_string();
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id.clone())),
        deposit = 1
    ).assert_success();

    // staker1 staking lpt 
    println!("----->> Staker1 staking lpt.");
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id.clone()).unwrap(), "0");

    // should panic when remove_user_rps_by_stakepool
    assert_err!(call!(
        staker1,
        stakepooling.remove_user_rps_by_stakepool("swap".to_string()),
        deposit = 0
    ), "E42: invalid stakepool id");

    assert_eq!(call!(
        staker1,
        stakepooling.remove_user_rps_by_stakepool(stakepool_id.clone())
    ).unwrap_json::<bool>(), false);

    //The rewards have been handed out, but stakepool not expire
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60 * 11);

    assert_eq!(show_unclaim(&stakepooling, staker1.account_id(), stakepool_id.clone(), false).0, to_yocto("10"));
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);

    call!(
        owner,
        stakepooling.force_clean_stakepool(stakepool_id.clone())
    ).assert_success();

    let stakepool_info = show_outdated_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Cleared".to_string(), to_yocto("10"), 0, 0, to_yocto("10"), 0, to_yocto("10"));

    call!(
        staker1,
        stakepooling.remove_user_rps_by_stakepool(stakepool_id.clone())
    ).assert_success();

    assert!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id.clone()).is_none());
}

#[test]
fn test_claim_reward_by_stakepool(){
    generate_user_account!(root, owner, staker1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1]);

    let stakepool_id = "swap@0#0".to_string();
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    

    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id.clone())),
        deposit = 1
    ).assert_success();

    assert_err!(call!(
        staker1,
        stakepooling.claim_reward_by_stakepool(stakepool_id.clone())
    ), "E10: account not registered");

    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    // staker1 staking lpt 
    println!("----->> Staker1 staking lpt.");
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60);

    assert_err!(call!(
        staker1,
        stakepooling.claim_reward_by_stakepool("random".to_string())
    ), "E42: invalid stakepool id");

    call!(
        staker1,
        stakepooling.claim_reward_by_stakepool(stakepool_id.clone()),
        deposit = 0
    ).assert_success();

    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);
}

#[test]
fn test_claim_reward_by_locktoken(){
    generate_user_account!(root, owner, staker1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1]);

    let stakepool_id1 = format!("{}@0#0", pool.account_id());
    let stakepool_id2 = format!("{}@0#1", pool.account_id());
    let locktoken_id = format!("{}@0", pool.account_id());
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    
    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("20"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id1.clone())),
        deposit = 1
    ).assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id2.clone())),
        deposit = 1
    ).assert_success();

    assert_err!(call!(
        staker1,
        stakepooling.claim_reward_by_locktoken(locktoken_id.clone())
    ), "E10: account not registered");

    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    // staker1 staking lpt 
    println!("----->> Staker1 staking lpt.");
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60);

    call!(
        staker1,
        stakepooling.claim_reward_by_locktoken(locktoken_id.clone()),
        deposit = 0
    ).assert_success();

    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id1.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);

    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id2.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);
}

#[test]
fn test_withdraw_reward(){
    generate_user_account!(root, owner, staker1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1]);

    let stakepool_id = "swap@0#0".to_string();
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());

    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id.clone())),
        deposit = 1
    ).assert_success();

    assert_err!(call!(
        staker1,
        stakepooling.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    ), "E10: account not registered");

    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    // staker1 staking lpt 
    println!("----->> Staker1 staking lpt.");
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60);

    assert_err!(call!(
        staker1,
        stakepooling.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    ), "E21: token not registered");

    call!(
        staker1,
        stakepooling.claim_reward_by_stakepool(stakepool_id.clone()),
        deposit = 0
    ).assert_success();

    assert_err!(call!(
        staker1,
        stakepooling.withdraw_reward(token1.valid_account_id(), Some(U128(to_yocto("1.1")))),
        deposit = 1
    ), "E22: not enough tokens in deposit");
}
use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};

use nstable_stakepooling_v2::HRSimpleStakePoolTerms;

use crate::common::utils::*;
use crate::common::init::deploy_stakepooling;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_user_rps() {
    generate_user_account!(root, owner, staker1, staker2);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![&staker1, &staker2]);

    let stakepool_id1 = "swap@0#0".to_string();
    let stakepool_id2 = "swap@0#1".to_string();
    let locktoken_id = format!("{}@0", pool.account_id());
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(staker2, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let current_timestamp = root.borrow_runtime().cur_block.block_timestamp;

    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: to_sec(current_timestamp + to_nano(100)),
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward to stakepool1
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

    // staker1 staking lpt 
    println!("----->> Staker1 staking lpt.");
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id1.clone()).unwrap(), "0");

    //add stakepool2 after staker1 staking 
    call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: locktoken_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: to_sec(current_timestamp + to_nano(100)),
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward to stakepool2
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto("10")), None, generate_reward_msg(stakepool_id2.clone())),
        deposit = 1
    ).assert_success();

    assert!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id2.clone()).is_none());

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(60);

    call!(
        staker1,
        stakepooling.claim_reward_by_locktoken(locktoken_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id1.clone()).unwrap(), "0");
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id2.clone()).unwrap(), "0");

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(100 + 60);

    call!(
        staker1,
        stakepooling.claim_reward_by_locktoken(locktoken_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id1.clone()).unwrap(), to_yocto("1").to_string());
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id2.clone()).unwrap(), to_yocto("1").to_string());

    // staker2 staking lpt 
    println!("----->> Staker2 staking lpt.");
    call!(
        staker2,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id1.clone()).unwrap(), to_yocto("1").to_string());
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id2.clone()).unwrap(), to_yocto("1").to_string());

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(100 + 60 * 2);

    call!(
        staker1,
        stakepooling.claim_reward_by_locktoken(locktoken_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(), stakepool_id2.clone()).unwrap(), to_yocto("1.5").to_string());
    call!(
        staker2,
        stakepooling.claim_reward_by_locktoken(locktoken_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(), stakepool_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(), stakepool_id2.clone()).unwrap(), to_yocto("1.5").to_string());


    //withdraw all locktoken 
    call!(
        staker1,
        stakepooling.withdraw_locktoken(locktoken_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    assert!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id1.clone()).is_none());
    assert!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id2.clone()).is_none());
    call!(
        staker2,
        stakepooling.withdraw_locktoken(locktoken_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    assert!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id1.clone()).is_none());
    assert!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id2.clone()).is_none());

    //staking in the same session_interval
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id2.clone()).unwrap(), to_yocto("1.5").to_string());

    call!(
        staker2,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id2.clone()).unwrap(), to_yocto("1.5").to_string());

    //withdraw all locktoken again
    call!(
        staker1,
        stakepooling.withdraw_locktoken(locktoken_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    call!(
        staker2,
        stakepooling.withdraw_locktoken(locktoken_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(100 + 60 * 3);

    //staking in the next session_interval
    call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id1.clone()).unwrap(), "0");
    assert_eq!(get_user_rps(&stakepooling, staker1.account_id(),stakepool_id2.clone()).unwrap(), "0");

    call!(
        staker2,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id1.clone()).unwrap(), "0");
    assert_eq!(get_user_rps(&stakepooling, staker2.account_id(),stakepool_id2.clone()).unwrap(), "0");
}
use near_sdk_sim::{call, init_simulator, to_yocto};

use crate::common::utils::*;
use crate::common::views::*;
use crate::common::actions::*;

mod common;


/// staking, unstaking, staking again, half unstaking
/// append staking
#[test]
fn lpt_stake_unstake() {
    // prepair users
    generate_user_account!(root, owner, staker1);

    let (pool, token1, _) = prepair_pool_and_liquidity(&root, &owner, stakepooling_id(), vec![&staker1]);

    let (stakepooling, stakepool_id) = prepair_stakepool(&root, &owner, &token1, to_yocto("500"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);

    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        staker1,
        stakepooling.withdraw_locktoken(format!("{}@0", swap()), to_yocto("1").into()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert!(user_locktokens.get(&String::from("swap@0")).is_none());
    
    assert!(root.borrow_runtime_mut().produce_blocks(120).is_ok());
    let out_come = call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        staker1,
        stakepooling.withdraw_locktoken(format!("{}@0", swap()), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        staker1,
        stakepooling.withdraw_locktoken(format!("{}@0", swap()), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert!(user_locktokens.get(&String::from("swap@0")).is_none());

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        staker1,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker1.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 8, 7, to_yocto("7"), to_yocto("1"), to_yocto("3"));
    let unclaim = show_unclaim(&stakepooling, staker1.account_id(), stakepool_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let out_come = call!(
        staker1,
        stakepooling.claim_reward_by_stakepool(stakepool_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 8, 8, to_yocto("8"), 0, to_yocto("3"));
    let unclaim = show_unclaim(&stakepooling, staker1.account_id(), stakepool_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
}


use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use nstable_stakepooling_v2::{HRSimpleStakePoolTerms};

use crate::common::utils::*;
use crate::common::views::*;
use crate::common::actions::*;
use crate::common::init::deploy_stakepooling;

mod common;

#[test]
fn multi_stakepool_in_single_locktoken() {
    generate_user_account!(root, owner, staker);
    println!("----->> owner and staker prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // staker add liqidity 
    add_liquidity(&staker, &pool, &token1, &token2, 0);
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), to_va(staker.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by staker.");

    // create stakepool with token1
    let (stakepooling, stakepool_ids) = prepair_multi_stakepools(&root, &owner, &token1, to_yocto("10"), 32);
    let stakepool_id = stakepool_ids[stakepool_ids.len() - 1].clone();
    println!("----->> StakePool till {} is ready.", stakepool_id.clone());

    // register LP token to stakepooling contract
    call!(root, pool.mft_register(":0".to_string(), to_va(stakepooling_id())), deposit = to_yocto("1"))
    .assert_success();
    println!("----->> Registered LP 0 to {}.", stakepooling_id());
    // register staker to stakepooling contract and stake liquidity token
    call!(staker, stakepooling.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    println!("----->> Registered staker to {}.", stakepooling_id());
    call!(
        staker,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 0, 0, 0, 0, 0);
    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    show_locktokensinfo(&stakepooling, false);
    println!("----->> Staker staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height,
        );
        let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
        assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, to_yocto("1"), 0);
        let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool_id.clone(), false);
        assert_eq!(unclaim.0, to_yocto("1"));
    }

    // staker claim reward
    println!();
    println!("********** Staker claim reward by locktoken_id ************");

    let out_come = call!(
        staker,
        stakepooling.claim_reward_by_locktoken(String::from("swap@0")),
        deposit = 0
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    // println!(
    //     "profile_data: {:#?} \n\ntokens_burnt: {} Near", 
    //     out_come.profile_data(), 
    //     (out_come.tokens_burnt()) as f64 / 1e24
    // );
    println!("\ntokens_burnt: {} Near", (out_come.tokens_burnt()) as f64 / 1e24);
    println!("Gas_burnt: {} TGas \n", (out_come.gas_burnt()) as f64 / 1e12);
    // make sure the total gas is less then 300T
    assert!(out_come.gas_burnt() < 300 * u64::pow(10, 12));

    // println!("profile_data: {:#?} \n", out_come.profile_data());
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height,
        );
        let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
        assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 2, 1, to_yocto("1"), to_yocto("1"), 0);
        let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool_id.clone(), false);
        assert_eq!(unclaim.0, to_yocto("1"));
    }

    // add lptoken
    println!();
    println!("********** Staker add locktoken ************");
    call!(
        staker,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    println!("\ntokens_burnt: {} Near", (out_come.tokens_burnt()) as f64 / 1e24);
    println!("Gas_burnt: {} TGas \n", (out_come.gas_burnt()) as f64 / 1e12);

    let user_locktokens = show_user_locktoken_amounts(&stakepooling, staker.account_id(), false);
    assert_eq!(user_locktokens.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("10"), 2, 2, to_yocto("2"), 0, 0);
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Staker added locktoken at #{}.", root.borrow_runtime().current_block().block_height);

}

#[test]
fn multi_stakepool_with_different_state() {
    generate_user_account!(root, owner, staker);
    println!("----->> owner and staker prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // staker add liqidity 
    add_liquidity(&staker, &pool, &token1, &token2, 0);
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), to_va(staker.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by staker.");

    println!("----->> Deploying stakepooling contract.");
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    ).assert_success();

    println!("----->> Creating stakepool0.");
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool0_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool0_id = stakepoolid.clone();
    } else {
        stakepool0_id = String::from("N/A");
    }
    println!("    StakePool {} created at Height#{}", stakepool0_id.clone(), root.borrow_runtime().current_block().block_height);
    mint_token(&token1, &root, to_yocto("5000"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), to_yocto("500").into(), None, generate_reward_msg(stakepool0_id.clone())),
        deposit = 1
    ).assert_success();
    println!("    StakePool {} running at Height#{}", stakepool0_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("----->> Creating stakepool1.");
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool1_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool1_id = stakepoolid.clone();
    } else {
        stakepool1_id = String::from("N/A");
    }
    println!("    StakePool {} created at Height#{}", stakepool1_id.clone(), root.borrow_runtime().current_block().block_height);
    
    println!("----->> Creating stakepool2.");
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 300,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool2_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool2_id = stakepoolid.clone();
    } else {
        stakepool2_id = String::from("N/A");
    }
    println!("    StakePool {} created at Height#{}", stakepool2_id.clone(), root.borrow_runtime().current_block().block_height);
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), to_yocto("500").into(), None, generate_reward_msg(stakepool2_id.clone())),
        deposit = 1
    )
    .assert_success();
    println!("    StakePool {} deposit reward at Height#{}", stakepool2_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("---->> Registering LP 0 for {}.", stakepooling_id());
    call!(root, pool.mft_register(":0".to_string(), to_va(stakepooling_id())), deposit = to_yocto("1"))
    .assert_success();

    println!("---->> Step01: Staker register and stake liquidity token.");
    call!(staker, stakepooling.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    call!(
        staker,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("  Staker staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step02: Staker claiming reward by locktoken_id after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    call!(
        staker,
        stakepooling.claim_reward_by_locktoken(format!("{}@0", swap())),
        deposit = 0
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step03: Active stakepool1 after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 2, 1, to_yocto("1"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), to_yocto("500").into(), None, generate_reward_msg(stakepool1_id.clone())),
        deposit = 1
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    println!("    StakePool {} running at Height#{}", stakepool1_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("----->> Step04: Staker claiming reward by locktoken_id after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    call!(
        staker,
        stakepooling.claim_reward_by_locktoken(format!("{}@0", swap())),
        deposit = 0
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step05: Staker claiming reward by locktoken_id after 100 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(100).is_ok());
    println!("  Chain goes for 100 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 5, 3, to_yocto("3"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool2_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    call!(
        staker,
        stakepooling.claim_reward_by_locktoken(format!("{}@0", swap())),
        deposit = 0
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 5, 5, to_yocto("5"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool2_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

}

#[test]
fn multi_stakepool_with_different_state_cd_account() {
    generate_user_account!(root, owner, staker);
    println!("----->> owner and staker prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // staker add liqidity 
    add_liquidity(&staker, &pool, &token1, &token2, 0);
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), to_va(staker.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by staker.");

    println!("----->> Deploying stakepooling contract.");
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();

    println!("----->> Creating stakepool0.");
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool0_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool0_id = stakepoolid.clone();
    } else {
        stakepool0_id = String::from("N/A");
    }
    println!("    StakePool {} created at Height#{}", stakepool0_id.clone(), root.borrow_runtime().current_block().block_height);
    mint_token(&token1, &root, to_yocto("5000"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), to_yocto("500").into(), None, generate_reward_msg(stakepool0_id.clone())),
        deposit = 1
    )
    .assert_success();
    println!("    StakePool {} running at Height#{}", stakepool0_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("----->> Creating stakepool1.");
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool1_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool1_id = stakepoolid.clone();
    } else {
        stakepool1_id = String::from("N/A");
    }
    println!("    StakePool {} created at Height#{}", stakepool1_id.clone(), root.borrow_runtime().current_block().block_height);
    
    println!("----->> Creating stakepool2.");
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 300,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool2_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool2_id = stakepoolid.clone();
    } else {
        stakepool2_id = String::from("N/A");
    }
    println!("    StakePool {} created at Height#{}", stakepool2_id.clone(), root.borrow_runtime().current_block().block_height);
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), to_yocto("500").into(), None, generate_reward_msg(stakepool2_id.clone())),
        deposit = 1
    )
    .assert_success();
    println!("    StakePool {} deposit reward at Height#{}", stakepool2_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("---->> Registering LP 0 for {}.", stakepooling_id());
    call!(root, pool.mft_register(":0".to_string(), to_va(stakepooling_id())), deposit = to_yocto("1"))
    .assert_success();

    call!(
        owner,
        stakepooling.modify_cd_strategy_item(0, 1000, 10_000),
        deposit = 1
    ).assert_success();

    println!("---->> Step01: Staker register and stake liquidity token.");
    call!(staker, stakepooling.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        staker,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("0.5").into(), None, generate_cd_account_msg(0, 0)),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("  Staker staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step02: Staker claiming reward by locktoken_id after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    call!(
        staker,
        stakepooling.claim_reward_by_locktoken(format!("{}@0", swap()))
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step03: Active stakepool1 after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 2, 1, to_yocto("1"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), to_yocto("500").into(), None, generate_reward_msg(stakepool1_id.clone())),
        deposit = 1
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    println!("    StakePool {} running at Height#{}", stakepool1_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("----->> Step04: Staker claiming reward by locktoken_id after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    call!(
        staker,
        stakepooling.claim_reward_by_locktoken(format!("{}@0", swap())),
        deposit = 0
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step05: Staker claiming reward by locktoken_id after 100 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(100).is_ok());
    println!("  Chain goes for 100 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 5, 3, to_yocto("3"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool2_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    call!(
        staker,
        stakepooling.claim_reward_by_locktoken(format!("{}@0", swap())),
        deposit = 0
    ).assert_success();
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool0_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 5, 5, to_yocto("5"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool1_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool2_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&stakepooling, staker.account_id(), stakepool2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Staker claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

}
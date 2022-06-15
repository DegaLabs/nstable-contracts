
use near_sdk::json_types::{U128};
use near_sdk::{Balance};
use near_sdk_sim::{call, to_yocto, ContractAccount, UserAccount};

// use near_sdk_sim::transaction::ExecutionStatus;
use nstable_exchange::{ContractContract as TestnStable};
use test_token::ContractContract as TestToken;
use nstable_stakepooling_v2::{ContractContract as StakePooling};
use nstable_stakepooling_v2::{HRSimpleStakePoolTerms};
use near_sdk::serde_json::Value;

use super::init::*;
use super::utils::*;

#[allow(dead_code)]
pub(crate) fn prepair_pool_and_liquidity(
    root: &UserAccount, 
    owner: &UserAccount,
    stakepooling_id: String,
    lps: Vec<&UserAccount>,
) -> (ContractAccount<TestnStable>, ContractAccount<TestToken>, ContractAccount<TestToken>) {
    let pool = deploy_pool(&root, swap(), owner.account_id());
    let token1 = deploy_token(&root, dai(), vec![swap()]);
    let token2 = deploy_token(&root, eth(), vec![swap()]);
    call!(owner, pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())]), deposit=1)
    .assert_success();
    call!(root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    ).assert_success();
    call!(root, pool.mft_register(":0".to_string(), to_va(stakepooling_id)), deposit = to_yocto("1"))
    .assert_success();
    for lp in lps {
        add_liquidity(lp, &pool, &token1, &token2, 0);
    }
    (pool,token1, token2)
}

#[allow(dead_code)]
pub(crate) fn prepair_pool(
    root: &UserAccount, 
    owner: &UserAccount, 
) -> (ContractAccount<TestnStable>, ContractAccount<TestToken>, ContractAccount<TestToken>) {
    let pool = deploy_pool(&root, swap(), owner.account_id());
    let token1 = deploy_token(&root, dai(), vec![swap()]);
    let token2 = deploy_token(&root, eth(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())]),
        deposit=1
    );
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    (pool, token1, token2)
}

#[allow(dead_code)]
pub(crate) fn prepair_stakepool(
    root: &UserAccount, 
    owner: &UserAccount,
    token: &ContractAccount<TestToken>,
    total_reward: Balance,
) -> (ContractAccount<StakePooling>, String) {
    // create stakepool
    
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", swap()),
            reward_token: to_va(token.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let stakepool_id: String;
    if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
        stakepool_id = stakepoolid.clone();
    } else {
        stakepool_id = String::from("N/A");
    }
    // println!("    StakePool {} created at Height#{}", stakepool_id.clone(), root.borrow_runtime().current_block().block_height);
    
    // deposit reward token
    call!(
        root,
        token.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token, &root, total_reward.into());
    call!(
        root,
        token.ft_transfer_call(to_va(stakepooling_id()), total_reward.into(), None, generate_reward_msg(stakepool_id.clone())),
        deposit = 1
    )
    .assert_success();
    // println!("    StakePool running at Height#{}", root.borrow_runtime().current_block().block_height);

    (stakepooling, stakepool_id)
}

#[allow(dead_code)]
pub(crate) fn prepair_multi_stakepools(
    root: &UserAccount, 
    owner: &UserAccount,
    token: &ContractAccount<TestToken>,
    total_reward: Balance,
    stakepool_count: u32,
) -> (ContractAccount<StakePooling>, Vec<String>) {
    // create stakepools
    
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    let mut stakepool_ids: Vec<String> = vec![];

    // register stakepooling contract to reward token
    call!(
        root,
        token.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();

    mint_token(&token, &root, to_yocto("100000"));

    for _ in 0..stakepool_count {
        let out_come = call!(
            owner,
            stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
                locktoken_id: format!("{}@0", swap()),
                reward_token: to_va(token.account_id()),
                start_at: 0,
                reward_per_session: to_yocto("1").into(),
                session_interval: 60,
            }, Some(U128(1000000000000000000))),
            deposit = to_yocto("1")
        );
        out_come.assert_success();
        let stakepool_id: String;
        if let Value::String(stakepoolid) = out_come.unwrap_json_value() {
            stakepool_id = stakepoolid.clone();
        } else {
            stakepool_id = String::from("N/A");
        }
        call!(
            root,
            token.ft_transfer_call(to_va(stakepooling_id()), total_reward.into(), None, generate_reward_msg(stakepool_id.clone())),
            deposit = 1
        )
        .assert_success();

        stakepool_ids.push(stakepool_id.clone());

        println!("  StakePool {} created and running at Height#{}", stakepool_id.clone(), root.borrow_runtime().current_block().block_height);
    }
    if stakepool_count >= 32 {
        let out_come = call!(
            owner,
            stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
                locktoken_id: format!("{}@0", swap()),
                reward_token: to_va(token.account_id()),
                start_at: 0,
                reward_per_session: to_yocto("1").into(),
                session_interval: 60,
            }, Some(U128(1000000000000000000))),
            deposit = to_yocto("1")
        );
        assert!(!out_come.is_ok());
        let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
        assert!(ex_status.contains("E36: the number of stakepools has reached its limit"));
    }
    
    (stakepooling, stakepool_ids)
}

pub(crate) fn add_liquidity(
    user: &UserAccount, 
    pool: &ContractAccount<TestnStable>, 
    token1: &ContractAccount<TestToken>, 
    token2: &ContractAccount<TestToken>, 
    pool_id: u64,
) {
    mint_token(&token1, user, to_yocto("105"));
    mint_token(&token2, user, to_yocto("105"));
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("100").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        token2.ft_transfer_call(to_va(swap()), to_yocto("100").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        pool.add_liquidity(pool_id, vec![U128(to_yocto("100")), U128(to_yocto("100"))], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();
}

pub(crate) fn mint_token(token: &ContractAccount<TestToken>, user: &UserAccount, amount: Balance) {
    // call!(
    //     user,
    //     token.storage_deposit(None, None),
    //     deposit = to_yocto("1")
    // )
    // .assert_success();
    call!(
        user,
        token.mint(to_va(user.account_id.clone()), amount.into())
    ).assert_success();
}

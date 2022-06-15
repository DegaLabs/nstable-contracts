use near_sdk_sim::{call, init_simulator, to_yocto, view};
use nstable_stakepooling_v2::{CDStrategyInfo};

use crate::common::utils::*;
use crate::common::init::deploy_stakepooling;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
pub fn test_strategy(){
    println!("----->> Prepare accounts.");
    generate_user_account!(root, owner);
    println!("<<----- owner prepared.");

    println!("----->> Prepare nstable-exchange and swap pool.");
    let (_pool, _token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![]);
    println!("<<----- The pool prepaired.");

    // deploy stakepooling contract and register user
    println!("----->> Deploy stakepooling.");
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    let strategy_info = view!(stakepooling.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    for index in 0..32{
        assert_strategy(&strategy_info, index, 0, 0, false, 0);
    }

    call!(
        owner,
        stakepooling.modify_cd_strategy_item(0, 100, 10),
        deposit = 1
    ).assert_success();
    let strategy_info = view!(stakepooling.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 100, 10, true, 0);

    call!(
        owner,
        stakepooling.modify_cd_strategy_item(0, 200, 20),
        deposit = 1
    ).assert_success();
    let strategy_info = view!(stakepooling.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 200, 20, true, 0);

    call!(
        owner,
        stakepooling.modify_cd_strategy_item(0, 0, 20),
        deposit = 1
    ).assert_success();
    let strategy_info = view!(stakepooling.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 0, 0, false, 0);

    call!(
        owner,
        stakepooling.modify_default_locktoken_slash_rate(20),
        deposit = 1
    ).assert_success();
    let strategy_info = view!(stakepooling.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 0, 0, false, 20);
    
}

#[test]
pub fn test_operators(){
    println!("----->> Prepare accounts.");
    generate_user_account!(root, owner, staker1, staker2);
    println!("<<----- owner prepared.");

    println!("----->> Prepare nstable-exchange and swap pool.");
    let (_pool, _token1, _) = prepair_pool_and_liquidity(
        &root, &owner, stakepooling_id(), vec![]);
    println!("<<----- The pool prepaired.");

    // deploy stakepooling contract and register user
    println!("----->> Deploy stakepooling.");
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());

    call!(
        owner,
        stakepooling.extend_operators(vec![staker1.valid_account_id(), staker2.valid_account_id()]),
        deposit = 1
    ).assert_success();

    assert_eq!(get_metadata(&stakepooling).operators, vec!["staker1", "staker2"]);

    call!(
        owner,
        stakepooling.remove_operators(vec![staker2.valid_account_id()]),
        deposit = 1
    ).assert_success();
    assert_eq!(get_metadata(&stakepooling).operators, vec!["staker1"]);
}
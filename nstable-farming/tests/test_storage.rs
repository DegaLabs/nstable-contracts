use near_sdk_sim::{call, init_simulator, to_yocto};

use crate::common::utils::*;
use crate::common::init::deploy_stakepooling;


mod common;

#[test]
fn storage_stake() {
    generate_user_account!(root, owner, staker1, staker2);

    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());

    // staker1 register
    assert_err!(call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("0.01")),
        "E11: insufficient $NEAR storage deposit");

    let orig_user_balance = staker1.account().unwrap().amount;
    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("0.1")).assert_success();
    assert!(orig_user_balance - staker1.account().unwrap().amount > to_yocto("0.1"));
    assert!(orig_user_balance - staker1.account().unwrap().amount < to_yocto("0.11"));

    // staker1 repeat register
    let orig_user_balance = staker1.account().unwrap().amount;
    call!(staker1, stakepooling.storage_deposit(None, None), deposit = to_yocto("0.1")).assert_success();
    assert!(orig_user_balance - staker1.account().unwrap().amount < to_yocto("0.001"));

    // staker1 withdraw storage
    let out_come = call!(staker1, stakepooling.storage_withdraw(None), deposit = 1);
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E14: no storage can withdraw"));

    // staker1 unregister storage
    let orig_user_balance = staker1.account().unwrap().amount;
    call!(staker1, stakepooling.storage_unregister(None), deposit = 1).assert_success();
    assert!(staker1.account().unwrap().amount - orig_user_balance > to_yocto("0.09"));
    assert!(staker1.account().unwrap().amount - orig_user_balance < to_yocto("0.1"));

    // staker1 help staker2 register
    let orig_user_balance = staker1.account().unwrap().amount;
    let orig_user_balance_famer2 = staker2.account().unwrap().amount;
    let out_come = call!(staker1, stakepooling.storage_deposit(Some(to_va(staker2.account_id())), None), deposit = to_yocto("1"));
    out_come.assert_success();
    assert!(orig_user_balance - staker1.account().unwrap().amount > to_yocto("0.1"));
    assert!(orig_user_balance - staker1.account().unwrap().amount < to_yocto("0.11"));
    assert!(orig_user_balance_famer2 - staker2.account().unwrap().amount == 0);
}
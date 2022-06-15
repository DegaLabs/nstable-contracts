use std::convert::TryFrom;

use near_sdk::json_types::ValidAccountId;
use near_sdk_sim::{deploy, init_simulator, to_yocto};
use crate::common::views::*;

pub mod common;

use nstable_stakepooling_v2::ContractContract as StakePooling;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PREV_STAKEPOOLING_WASM_BYTES => "../res/nstable_stakepooling_v2_v200.wasm",
    STAKEPOOLING_WASM_BYTES => "../res/nstable_stakepooling_v2_release.wasm",
}


#[test]
fn test_upgrade() {
    let root = init_simulator(None);
    let test_user = root.create_user("test".to_string(), to_yocto("100"));
    let stakepooling = deploy!(
        contract: StakePooling,
        contract_id: "stakepooling".to_string(),
        bytes: &PREV_STAKEPOOLING_WASM_BYTES,
        signer_account: root,
        init_method: new(ValidAccountId::try_from(root.account_id.clone()).unwrap())
    );

    // Failed upgrade with no permissions.
    let result = test_user
        .call(
            stakepooling.user_account.account_id.clone(),
            "upgrade",
            &STAKEPOOLING_WASM_BYTES,
            near_sdk_sim::DEFAULT_GAS,
            0,
        )
        .status();
    assert!(format!("{:?}", result).contains("ERR_NOT_ALLOWED"));

    // Upgrade with calling migration. 
    root.call(
        stakepooling.user_account.account_id.clone(),
        "upgrade",
        &STAKEPOOLING_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();
    let metadata = get_metadata(&stakepooling);
    // println!("{:#?}", metadata);
    assert_eq!(metadata.version, "2.1.6".to_string());

    // Upgrade to the same code without migration is successful.
    let out_come = root.call(
        stakepooling.user_account.account_id.clone(),
        "upgrade",
        &STAKEPOOLING_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    );
    out_come.assert_success();
    // let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    // assert!(ex_status.contains("not implemented"));
    let metadata = get_metadata(&stakepooling);
    assert_eq!(metadata.version, "2.1.6".to_string());
}

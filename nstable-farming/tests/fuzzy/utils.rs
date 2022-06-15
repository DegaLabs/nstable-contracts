use near_sdk::json_types::{U128};
use near_sdk::{Balance, AccountId};
use near_sdk_sim::{call, deploy, view, init_simulator, to_yocto, ContractAccount, UserAccount};
// use near_sdk_sim::transaction::ExecutionStatus;
use nstable_exchange::{ContractContract as TestnStable};
use test_token::ContractContract as TestToken;
use nstable_stakepooling_v2::{HRSimpleStakePoolTerms, ContractContract as StakePooling, StakePoolInfo};
use near_sdk::serde_json::Value;
use near_sdk::json_types::{ValidAccountId};
use std::convert::TryFrom;
use std::collections::HashMap;
use crate::fuzzy::{
    constant::*,
    types::*,
};



near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/nstable_exchange_release.wasm",
    STAKEPOOL_WASM_BYTES => "../res/nstable_stakepooling_v2_release.wasm",
}

pub fn deploy_stakepooling(root: &UserAccount, stakepooling_id: AccountId, owner_id: AccountId) -> ContractAccount<StakePooling> {
    let stakepooling = deploy!(
        contract: StakePooling,
        contract_id: stakepooling_id,
        bytes: &STAKEPOOL_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va(owner_id))
    );
    stakepooling
}

pub fn deploy_pool(root: &UserAccount, contract_id: AccountId, owner_id: AccountId) -> ContractAccount<TestnStable> {
    let pool = deploy!(
        contract: TestnStable,
        contract_id: contract_id,
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va(owner_id), 4, 1)
    );
    pool
}

pub fn deploy_token(
    root: &UserAccount,
    token_id: AccountId,
    accounts_to_register: Vec<AccountId>,
) -> ContractAccount<TestToken> {
    let t = deploy!(
        contract: TestToken,
        contract_id: token_id,
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, t.new()).assert_success();
    for account_id in accounts_to_register {
        call!(
            root,
            t.storage_deposit(Some(to_va(account_id)), None),
            deposit = to_yocto("1")
        )
        .assert_success();
    }
    t
}


pub fn dai() -> AccountId {
    "dai".to_string()
}

pub fn eth() -> AccountId {
    "eth".to_string()
}

pub fn swap() -> AccountId {
    "swap".to_string()
}

pub fn stakepooling_id() -> AccountId {
    "stakepooling".to_string()
}

pub fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

pub fn prepair_env(
) -> (UserAccount, UserAccount, ContractAccount<StakePooling>, ContractAccount<TestnStable>, Vec<Operator>) {

    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let staker_stake = root.create_user("staker_stake".to_string(), to_yocto("100"));
    let staker_unstake = root.create_user("staker_unstake".to_string(), to_yocto("100"));
    let staker_claim = root.create_user("staker_claim".to_string(), to_yocto("100"));
    println!("<<----- owner and 3 stakers prepared.");

    println!("----->> Deploy stakepooling and register stakers.");
    let stakepooling = deploy_stakepooling(&root, stakepooling_id(), owner.account_id());
    call!(staker_stake, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(staker_unstake, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(staker_claim, stakepooling.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- stakepooling deployed, stakers registered.");

    let pool = deploy_pool(&root, swap(), owner.account_id());
    let token1 = deploy_token(&root, dai(), vec![swap()]);
    let token2 = deploy_token(&root, eth(), vec![swap()]);
    call!(owner, pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())]), deposit=1)
    .assert_success();

    call!(root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    ).assert_success();

    call!(root, pool.mft_register(":0".to_string(), to_va(stakepooling_id())), deposit = to_yocto("1"))
    .assert_success();

    add_liqudity(&staker_stake, &pool, &token1, &token2, 0);
    add_liqudity(&staker_unstake, &pool, &token1, &token2, 0);
    add_liqudity(&staker_claim, &pool, &token1, &token2, 0);
    call!(
        staker_stake,
        pool.add_liquidity(0, vec![to_yocto(&(10 * OPERATION_NUM).to_string()).into(), to_yocto(&(10 * OPERATION_NUM).to_string()).into()], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    println!("----->> Create stakepool.");
    let stakepool_id = STAKEPOOL_ID.to_string();
    let out_come = call!(
        owner,
        stakepooling.create_simple_stakepool(HRSimpleStakePoolTerms{
            locktoken_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(Value::String(stakepool_id.clone()), out_come.unwrap_json_value());
    println!("<<----- StakePool {} created at #{}, ts:{}.", 
    stakepool_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> Deposit reward to turn stakepool Running.");
    call!(
        root,
        token1.storage_deposit(Some(to_va(stakepooling_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto(&OPERATION_NUM.to_string()));
    call!(
        root,
        token1.ft_transfer_call(to_va(stakepooling_id()), U128(to_yocto(&OPERATION_NUM.to_string())), None, format!("{{\"Reward\": {{\"stakepool_id\": \"{}\"}}}}", stakepool_id.clone())),
        deposit = 1
    )
    .assert_success();
    show_stakepoolinfo(&stakepooling, stakepool_id.clone(), true);
    println!("<<----- StakePool {} deposit reward at #{}, ts:{}.", 
    stakepool_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    (root, owner, stakepooling, pool, vec![Operator{user: staker_stake, preference: Preference::Stake}, Operator{user: staker_unstake, preference: Preference::Unstake}, Operator{user: staker_claim, preference: Preference::Claim}])
}

pub fn add_liqudity(
    user: &UserAccount, 
    pool: &ContractAccount<TestnStable>, 
    token1: &ContractAccount<TestToken>, 
    token2: &ContractAccount<TestToken>, 
    pool_id: u64,
) {
    mint_token(&token1, user, to_yocto(&(100 * OPERATION_NUM).to_string()));
    mint_token(&token2, user, to_yocto(&(100 * OPERATION_NUM).to_string()));
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto(&(100 * OPERATION_NUM).to_string()).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        token2.ft_transfer_call(to_va(swap()), to_yocto(&(100 * OPERATION_NUM).to_string()).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        pool.add_liquidity(pool_id, vec![U128(to_yocto("10")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();
}

pub fn mint_token(token: &ContractAccount<TestToken>, user: &UserAccount, amount: Balance) {
    call!(
        user,
        token.mint(to_va(user.account_id.clone()), amount.into())
    ).assert_success();
}

pub fn show_stakepoolinfo(
    stakepooling: &ContractAccount<StakePooling>,
    stakepool_id: String,
    show_print: bool,
) -> StakePoolInfo {
    let stakepool_info = get_stakepoolinfo(stakepooling, stakepool_id);
    if show_print {
        println!("StakePool Info ===>");
        println!(
            "  ID:{}, Status:{}, LockToken:{}, Reward:{}",
            stakepool_info.stakepool_id, stakepool_info.stakepool_status, stakepool_info.locktoken_id, stakepool_info.reward_token
        );
        println!(
            "  StartAt:{}, SessionReward:{}, SessionInterval:{}",
            stakepool_info.start_at, stakepool_info.reward_per_session.0, stakepool_info.session_interval
        );
        println!(
            "  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}",
            stakepool_info.total_reward.0,
            stakepool_info.claimed_reward.0,
            stakepool_info.unclaimed_reward.0,
            stakepool_info.last_round,
            stakepool_info.cur_round
        );
    }
    stakepool_info
}

fn get_stakepoolinfo(stakepooling: &ContractAccount<StakePooling>, stakepool_id: String) -> StakePoolInfo {
    view!(stakepooling.get_stakepool(stakepool_id)).unwrap_json::<StakePoolInfo>()
}

pub fn show_user_locktoken_amounts(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    show_print: bool,
) -> HashMap<String, U128> {
    let ret = view!(stakepooling.list_user_locktoken_amounts(to_va(user_id.clone())))
        .unwrap_json::<HashMap<String, U128>>();
    if show_print {
        println!("User LockTokens for {}: {:#?}", user_id, ret);
    }
    ret
}

pub(crate) fn show_unclaim(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    stakepool_id: String,
    show_print: bool,
) -> U128 {
    let stakepool_info = get_stakepoolinfo(stakepooling, stakepool_id.clone());
    let ret = view!(stakepooling.get_unclaimed_reward(to_va(user_id.clone()), stakepool_id.clone()))
        .unwrap_json::<U128>();
    if show_print {
        println!(
            "User Unclaimed for {}@{}:[CRR:{}, LRR:{}] {}",
            user_id, stakepool_id, stakepool_info.cur_round, stakepool_info.last_round, ret.0
        );
    }
    ret
}

pub fn assert_stakepooling(
    stakepool_info: &StakePoolInfo,
    stakepool_status: String,
    total_reward: u128,
    cur_round: u32,
    last_round: u32,
    claimed_reward: u128,
    unclaimed_reward: u128,
    beneficiary_reward: u128,
) {
    assert_eq!(stakepool_info.stakepool_status, stakepool_status);
    assert_eq!(stakepool_info.total_reward.0, total_reward);
    assert_eq!(stakepool_info.cur_round, cur_round);
    assert_eq!(stakepool_info.last_round, last_round);
    assert_eq!(stakepool_info.claimed_reward.0, claimed_reward);
    assert_eq!(stakepool_info.unclaimed_reward.0, unclaimed_reward);
    assert_eq!(stakepool_info.beneficiary_reward.0, beneficiary_reward);
}
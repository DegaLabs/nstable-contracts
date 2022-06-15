use near_sdk_sim::{
    view, ContractAccount, UserAccount,to_yocto
};
use rand::{Rng, LockTokenableRng};
use rand_pcg::Pcg32;
use nstable_stakepooling_v2::{ContractContract as StakePooling, StakePoolInfo};
use nstable_exchange::{ContractContract as TestnStable};
mod fuzzy;
use fuzzy::{
    utils::*,
    types::*,
    stake::*,
    unstake::*,
    claim::*,
    constant::*
};

pub fn get_operator<'a>(rng: &mut Pcg32, users: &'a Vec<Operator>) -> &'a Operator{
    let user_index = rng.gen_range(0..users.len());
    &users[user_index]
}

pub fn do_operation(ctx: &mut StakePoolInfo, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, stakepooling :&ContractAccount<StakePooling>, pool :&ContractAccount<TestnStable>){
    println!("locktokeninfo -- {:?}", view!(stakepooling.get_locktoken_info(format!("{}@0", pool.account_id()))).unwrap_json::<LockTokenInfo>());
    println!("stakepoolinfo -- {:?}", view!(stakepooling.get_stakepool(STAKEPOOL_ID.to_string())).unwrap_json::<StakePoolInfo>());
    match operator.preference{
        Preference::Stake => {
            do_stake(ctx, rng, root, operator, stakepooling, pool);
        },
        Preference::Unstake => {
            do_unstake(ctx, rng, root, operator, stakepooling, pool);
        },
        Preference::Claim => {
            do_claim(ctx, rng, root, operator, stakepooling, pool);
        },
    }
    println!("----->> move to 60 secs later.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("<<----- Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    
    if view!(stakepooling.get_locktoken_info(format!("{}@0", pool.account_id()))).unwrap_json::<LockTokenInfo>().amount.0 == 0{
        ctx.claimed_reward.0 += to_yocto("1");
        ctx.beneficiary_reward.0 += to_yocto("1");
    }else{
        ctx.unclaimed_reward.0 += to_yocto("1");
    }
}


fn generate_fuzzy_locktoken() -> Vec<u64>{
    let mut locktokens:Vec<u64> = Vec::new();

    let mut rng = rand::thread_rng();
    for _ in 0..FUZZY_NUM {
        let locktoken: u64 = rng.gen();
        locktokens.push(locktoken);
    }
    locktokens
}

#[test]
fn test_fuzzy(){

    let locktokens = generate_fuzzy_locktoken();
    for locktoken in locktokens {

        println!("*********************************************");
        println!("current locktoken : {}", locktoken);
        println!("*********************************************");

        let (root, _owner, stakepooling, pool, users) = prepair_env();

        let mut rng = Pcg32::locktoken_from_u64(locktoken as u64);
        let mut ctx = view!(stakepooling.get_stakepool(STAKEPOOL_ID.to_string())).unwrap_json::<StakePoolInfo>().clone();
        for i in 0..OPERATION_NUM{
            let operator = get_operator(&mut rng, &users);
            println!("NO.{} : {:?}", i, operator);
            do_operation(&mut ctx, &mut rng, &root, operator, &stakepooling, &pool);
        }
        let stakepool_info = show_stakepoolinfo(&stakepooling, STAKEPOOL_ID.to_string(), false);
        assert_stakepooling(&stakepool_info, "Ended".to_string(), to_yocto(&OPERATION_NUM.to_string()), ctx.cur_round, ctx.last_round, ctx.claimed_reward.0, ctx.unclaimed_reward.0, ctx.beneficiary_reward.0);
    }
}
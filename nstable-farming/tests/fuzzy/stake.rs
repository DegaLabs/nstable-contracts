use near_sdk_sim::{call, to_yocto, ContractAccount, UserAccount};
use nstable_stakepooling_v2::{ContractContract as StakePooling, StakePoolInfo};
use nstable_exchange::{ContractContract as TestnStable};
use rand_pcg::Pcg32;
use crate::fuzzy::{
    constant::*,
    utils::*,
    types::*,
};

pub fn do_stake(ctx: &mut StakePoolInfo, _rng: &mut Pcg32, root: &UserAccount, operator: &Operator, stakepooling :&ContractAccount<StakePooling>, pool :&ContractAccount<TestnStable>){
    let stakepool_id = STAKEPOOL_ID.to_string();
    let unclaim = show_unclaim(&stakepooling, operator.user.account_id(), stakepool_id.clone(), false);
    println!("----->> {} staking lpt.", operator.user.account_id());
    let out_come = call!(
        operator.user,
        pool.mft_transfer_call(":0".to_string(), to_va(stakepooling_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    println!("<<----- {} staked liquidity at #{}, ts:{}.", 
    operator.user.account_id(),
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    ctx.claimed_reward.0 += unclaim.0;
    ctx.unclaimed_reward.0 -= unclaim.0;
    ctx.last_round = ctx.cur_round;
    
    let stakepool_info = show_stakepoolinfo(&stakepooling, stakepool_id.clone(), false);
    assert_stakepooling(&stakepool_info, "Running".to_string(), to_yocto(&OPERATION_NUM.to_string()), ctx.cur_round, ctx.last_round, ctx.claimed_reward.0, ctx.unclaimed_reward.0, ctx.beneficiary_reward.0);
    ctx.cur_round += 1;
}
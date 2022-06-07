//! Implement all the relevant logic for owner of this contract.
use crate::*;
use near_sdk::{
    AccountId, assert_one_yocto
};
#[near_bindgen]
impl Contract {
    /// Change owner. Only can be called by owner.
    #[payable]
    pub fn set_owner(&mut self, owner_id: AccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.owner_id = owner_id.clone();
    }

    /// Get the owner of this account.
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Change state of contract, Only can be called by owner or guardians.
    #[payable]
    pub fn set_min_locked_amount(&mut self, min_locked: U128) {
        assert_one_yocto();
        self.assert_owner();

        self.min_locked_amount = min_locked.into();
    }

    #[payable]
    pub fn set_early_withdraw_penalty_rate(&mut self, penalty: u64) {
        assert_one_yocto();
        self.assert_owner();
        assert!(penalty <= MAX_WITHDRAWAL_PENALTY, "{}", ERR111_WITHDRAWAL_PENALTY_TOO_HIGH);
        self.early_withdraw_penalty_rate = penalty;
    }

    #[payable]
    pub fn set_penalty_collector(&mut self, collector: AccountId) {
        assert_one_yocto();
        self.assert_owner();

        self.penalty_collector = collector.clone();
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "{}", ERR100_NOT_ALLOWED
        );
    }

    /// Migration function from v2 to v2.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    // [AUDIT_09]
    #[private]
    pub fn migrate() -> Self {
        let contract: Contract = env::state_read().expect(ERR103_NOT_INITIALIZED);
        contract
    }
}


#[cfg(target_arch = "wasm32")]
mod upgrade {
    // use near_sdk::env::BLOCKCHAIN_INTERFACE;
    // use near_sdk::Gas;

    // use super::*;

    // const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";

    // /// Gas for calling migration call.
    // pub const GAS_FOR_MIGRATE_CALL: Gas = 5_000_000_000_000;

    // /// Self upgrade and call migrate, optimizes gas by not loading into memory the code.
    // /// Takes as input non serialized set of bytes of the code.
    // #[no_mangle]
    // pub extern "C" fn upgrade() {
    //     env::setup_panic_hook();
    //     env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
    //     let contract: Contract = env::state_read().expect(ERR103_NOT_INITIALIZED);
    //     contract.assert_owner();
    //     let current_id = env::current_account_id().into_bytes();
    //     let method_name = "migrate".as_bytes().to_vec();
    //     unsafe {
    //         BLOCKCHAIN_INTERFACE.with(|b| {
    //             // Load input into register 0.
    //             b.borrow()
    //                 .as_ref()
    //                 .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
    //                 .input(0);
    //             let promise_id = b
    //                 .borrow()
    //                 .as_ref()
    //                 .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
    //                 .promise_batch_create(current_id.len() as _, current_id.as_ptr() as _);
    //             b.borrow()
    //                 .as_ref()
    //                 .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
    //                 .promise_batch_action_deploy_contract(promise_id, u64::MAX as _, 0);
    //             let attached_gas = env::prepaid_gas() - env::used_gas() - GAS_FOR_MIGRATE_CALL;
    //             b.borrow()
    //                 .as_ref()
    //                 .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
    //                 .promise_batch_action_function_call(
    //                     promise_id,
    //                     method_name.len() as _,
    //                     method_name.as_ptr() as _,
    //                     0 as _,
    //                     0 as _,
    //                     0 as _,
    //                     attached_gas,
    //                 );
    //         });
    //     }
    // }

}

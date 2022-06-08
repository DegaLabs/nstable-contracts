use crate::event::NearEvent;
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use serde::Serialize;

/// Data to log for an FT mint event. To log this event, call [`.emit()`](FtMint::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
pub struct NaiBorrow<'a> {
    pub account_id: &'a AccountId,
    pub collateral_token_id: &'a AccountId,
    pub borrow_amount: &'a U128
}

impl NaiBorrow<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an FT mint event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`FtMint`] represents the data of each mint.
    pub fn emit_many(data: &[NaiBorrow<'_>]) {
        new_141_v1(Nep141EventKind::FtMint(data)).emit()
    }
}
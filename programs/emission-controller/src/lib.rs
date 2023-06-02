//! Gauge
//!

#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![deny(clippy::unwrap_used)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use vipers::prelude::*;

mod instructions;
mod macros;
mod state;

pub use instructions::*;
pub use state::*;

declare_id!("emi8Ahk86oEVsDdNAo31RxMMPQfCQEcxAvfCYAh8zfd");

#[program]
/// Smart wallet program.
pub mod emission_controller {
    use super::*;

    /// Calls [quarry_mine::quarry_mine::set_rewards_share].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_set_rewards_share(
        ctx: Context<DelegateSetRewardsShare>,
        new_share: u64,
    ) -> Result<()> {
        let operator = &ctx.accounts.with_delegate.operator;
        let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
        quarry::cpi::set_rewards_share(
            CpiContext::new_with_signer(
                ctx.accounts
                    .with_delegate
                    .quarry_mine_program
                    .to_account_info(),
                quarry::cpi::accounts::SetRewardsShare {
                    auth: ctx.accounts.with_delegate.to_auth_accounts(),
                    quarry: ctx.accounts.quarry.to_account_info(),
                },
                signer_seeds,
            ),
            new_share,
        )?;
        Ok(())
    }
}

/// Program errors.
#[error_code]
pub enum ErrorCode {
    #[msg("Signer is not authorized to perform this action.")]
    Unauthorized,
    #[msg("Pending authority must be set to the created operator.")]
    PendingAuthorityNotSet,
    #[msg("Operator is not the Rewarder authority.")]
    OperatorNotRewarderAuthority,
}

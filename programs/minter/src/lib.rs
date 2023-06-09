//! Proxy program for interacting with the token mint.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(deprecated)]

#[macro_use]
mod macros;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{self, Mint, TokenAccount};
use vipers::prelude::*;

mod instructions;
mod state;

use instructions::*;
pub use state::*;

declare_id!("MintpueFHs8fQMvd6uwuCBHhSGw3Wi7KtoMyEmjP29G");

#[program]
pub mod minter {
    use super::*;

    // --------------------------------
    // [MintWrapper] instructions
    // --------------------------------

    /// Creates a new [MintWrapper].
    #[access_control(ctx.accounts.validate())]
    pub fn new_wrapper(ctx: Context<NewWrapper>, hard_cap: u64) -> Result<()> {
        new_wrapper::handler(ctx, hard_cap)
    }

    /// Transfers admin to another account.
    #[access_control(ctx.accounts.validate())]
    pub fn transfer_admin(ctx: Context<TransferAdmin>) -> Result<()> {
        transfer_admin::handler(ctx)
    }

    /// Accepts the new admin.
    #[access_control(ctx.accounts.validate())]
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        accept_admin::handler(ctx)
    }

    // --------------------------------
    // [Minter] instructions
    // --------------------------------
    /// Creates a new [Minter].
    #[access_control(ctx.accounts.validate())]
    pub fn new_minter(ctx: Context<NewMinter>) -> Result<()> {
        instructions::new_minter::handler(ctx)
    }

    /// Updates a [Minter]'s allowance.
    #[access_control(ctx.accounts.validate())]
    pub fn minter_update(ctx: Context<MinterUpdate>, allowance: u64) -> Result<()> {
        minter_update::handler(ctx, allowance)
    }

    /// TODO: implement remove minter (close account)

    /// Performs a mint.
    #[access_control(ctx.accounts.validate())]
    pub fn perform_mint(ctx: Context<PerformMint>, amount: u64) -> Result<()> {
        perform_mint::handler(ctx, amount)
    }
}

/// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
    #[msg("Cannot mint over hard cap.")]
    HardcapExceeded,
    #[msg("Minter allowance exceeded.")]
    MinterAllowanceExceeded,
}

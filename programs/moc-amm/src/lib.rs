//! Proxy program for interacting with the token mint.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

mod instructions;
mod state;

use instructions::*;
pub use state::*;
use vipers::prelude::*;

declare_id!("mocJyKP7XPfSHs9q55ycWDfuWzQiqPAkoNXdabiW2Dc");

#[program]
pub mod moc_amm {
    use super::*;

    // --------------------------------
    // [MocAmm] instructions
    // --------------------------------

    /// Creates a new [MocAmm].    
    pub fn new_moc_amm(ctx: Context<NewMocAmm>, fee: u64, lp_mint: Pubkey) -> Result<()> {
        new_moc_amm::handler(ctx, fee, lp_mint)
    }
    pub fn claim_fee(ctx: Context<ClaimFee>, amount: u64) -> Result<()> {
        claim_fee::handler(ctx, amount)
    }
}

//! Liquidity mining rewards distribution program.
//!
//! The program consists of three types of accounts:
//!
//! - [Rewarder], which controls token rewards distribution
//! - [Quarry], which receive rewards, and
//! - [Miner], which stake tokens into [Quarry]s to receive rewards.
//!
//! This program is modeled after [Synthetix's StakingRewards.sol](https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol).
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(deprecated)]

#[macro_use]
mod macros;
mod state;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer};
use payroll::Payroll;
pub use state::*;
use vipers::prelude::*;

pub mod payroll;
pub mod quarry_state;
pub mod rewarder;

mod instructions;
pub use instructions::*;

use crate::quarry_state::StakeAction;

declare_id!("quaJeeoTrxSszHnpzNQrtFoF1kcnfMiSWuYpk8ddobB");

/// Maximum number of tokens that can be rewarded by a [Rewarder] per year.
pub const MAX_ANNUAL_REWARDS_RATE: u64 = u64::MAX >> 3;

/// Program for [quarry].
#[program]
pub mod quarry {
    use super::*;

    // --------------------------------
    // Rewarder Functions
    // --------------------------------

    /// Creates a new [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn new_rewarder(ctx: Context<NewRewarder>) -> Result<()> {
        new_rewarder::handler(ctx)
    }

    /// Set operator [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn set_operator(
        ctx: Context<MutableRewarderWithAuthority>,
        operator: Pubkey,
    ) -> Result<()> {
        mutable_rewarder_with_authority::handler_set_operator(ctx, operator)
    }

    /// Sets the pause authority.
    #[access_control(ctx.accounts.validate())]
    pub fn set_pause_authority(ctx: Context<SetPauseAuthority>) -> Result<()> {
        set_pause_authority::handler(ctx)
    }

    /// Pauses the [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn pause(ctx: Context<MutableRewarderWithPauseAuthority>) -> Result<()> {
        mutable_rewarder_with_pause_authority::handler(ctx, true)
    }

    /// Unpauses the [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn unpause(ctx: Context<MutableRewarderWithPauseAuthority>) -> Result<()> {
        mutable_rewarder_with_pause_authority::handler(ctx, false)
    }

    /// Transfers the [Rewarder] authority to a different account.
    #[access_control(ctx.accounts.validate())]
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        transfer_authority::handler(ctx, new_authority)
    }

    /// Accepts the authority to become the new rewarder.
    #[access_control(ctx.accounts.validate())]
    pub fn accept_authority(ctx: Context<AcceptAuthority>) -> Result<()> {
        accept_authority::handler(ctx)
    }

    /// Sets the amount of reward tokens distributed to all [Quarry]s per day.
    #[access_control(ctx.accounts.validate())]
    pub fn set_annual_rewards(ctx: Context<SetAnnualRewards>, new_rate: u64) -> Result<()> {
        set_annual_rewards::handler(ctx, new_rate)
    }

    // --------------------------------
    // Quarry functions
    // --------------------------------

    /// Creates a new [Quarry].
    /// This may only be called by the [Rewarder]::authority.    
    #[access_control(ctx.accounts.validate())]
    pub fn create_quarry(ctx: Context<CreateQuarry>) -> Result<()> {
        create_quarry::handler(ctx)
    }

    /// Sets the rewards share of a quarry.
    #[access_control(ctx.accounts.validate())]
    pub fn set_rewards_share(ctx: Context<SetRewardsShare>, new_share: u64) -> Result<()> {
        set_rewards_share::handler(ctx, new_share)
    }

    /// Sets the famine, which stops rewards.
    #[access_control(ctx.accounts.validate())]
    pub fn set_famine(ctx: Context<SetFamine>, famine_ts: i64) -> Result<()> {
        set_famine::handler(ctx, famine_ts)
    }

    /// Synchronizes quarry rewards with the rewarder.
    /// Anyone can call this.
    #[access_control(ctx.accounts.validate())]
    pub fn update_quarry_rewards(ctx: Context<UpdateQuarryRewards>) -> Result<()> {
        update_quarry_rewards::handler(ctx)
    }

    /// --------------------------------
    /// Miner functions
    /// --------------------------------

    /// Creates a [Miner] for the given authority.
    ///
    /// Anyone can call this; this is an associated account.
    #[access_control(ctx.accounts.validate())]
    pub fn create_miner(ctx: Context<CreateMiner>) -> Result<()> {
        create_miner::handler(ctx)
    }

    /// Claims rewards for the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        claim_rewards::handler(ctx)
    }

    /// Stakes tokens into the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn stake_tokens(ctx: Context<UserStake>, amount: u64) -> Result<()> {
        user_stake::handler_stake_tokens(ctx, amount)
    }

    /// Withdraws tokens from the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn withdraw_tokens(ctx: Context<UserStake>, amount: u64) -> Result<()> {
        user_stake::handler_withdraw_tokens(ctx, amount)
    }
}

/// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
    #[msg("Insufficient staked balance for withdraw request.")]
    InsufficientBalance,
    #[msg("Pending authority not set")]
    PendingAuthorityNotSet,
    #[msg("Invalid quarry rewards share")]
    InvalidRewardsShare,
    #[msg("Insufficient allowance.")]
    InsufficientAllowance,
    #[msg("New vault not empty.")]
    NewVaultNotEmpty,
    #[msg("Not enough tokens.")]
    NotEnoughTokens,
    #[msg("Invalid timestamp.")]
    InvalidTimestamp,
    #[msg("Max annual rewards rate exceeded.")]
    MaxAnnualRewardsRateExceeded,
    #[msg("Rewarder is paused.")]
    Paused,
    #[msg("Rewards earned exceeded quarry's upper bound.")]
    UpperboundExceeded,
}

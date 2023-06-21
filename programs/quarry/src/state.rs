//! State structs.

use crate::*;

/// Controls token rewards distribution to all [Quarry]s.
/// The [Rewarder] is also the [minter::Minter] registered to the [minter::MintWrapper].
#[account]
#[derive(Copy, Default, Debug)]
pub struct Rewarder {
    /// Random pubkey used for generating the program address.
    pub base: Pubkey,
    /// Bump seed for program address.
    pub bump: u8,

    /// Admin who controls the rewarder
    pub admin: Pubkey,
    /// Pending admin which must accept the admin
    pub pending_admin: Pubkey,

    /// Number of [Quarry]s the [Rewarder] manages.
    /// If more than this many [Quarry]s are desired, one can create
    /// a second rewarder.
    pub num_quarries: u16,
    /// Amount of reward tokens distributed per day
    pub annual_rewards_rate: u64,
    /// Total amount of rewards shares allocated to [Quarry]s
    pub total_rewards_shares: u64,
    /// Mint wrapper.
    pub mint_wrapper: Pubkey,
    /// Mint of the rewards token for this [Rewarder].
    pub rewards_token_mint: Pubkey,

    /// Authority allowed to pause a [Rewarder].
    pub pause_authority: Pubkey,
    /// If true, all instructions on the [Rewarder] are paused other than [quarry::unpause].
    pub is_paused: bool,

    // operator can set rewards share, normally it is gauge factory
    pub mint_authority: Pubkey,
}

impl Rewarder {
    /// Asserts that this [Rewarder] is not paused.
    pub fn assert_not_paused(&self) -> Result<()> {
        invariant!(!self.is_paused, Paused);
        Ok(())
    }
}

/// A pool which distributes tokens to its [Miner]s.
#[account]
#[derive(Copy, Default)]
pub struct Quarry {
    /// Rewarder which manages this quarry
    pub rewarder: Pubkey,
    /// Amm pool this quarry is designated to
    pub amm_pool: Pubkey,
    /// LP token this quarry is designated to
    pub token_mint_key: Pubkey,
    /// Bump.
    pub bump: u8,

    /// Index of the [Quarry].
    pub index: u16,
    /// Decimals on the token [Mint].
    // pub token_mint_decimals: u8, // This field is never used.
    /// Timestamp when quarry rewards cease
    pub famine_ts: i64,
    /// Timestamp of last checkpoint
    pub last_update_ts: i64,
    /// Rewards per token stored in the quarry
    pub rewards_per_token_stored: u128,
    /// Amount of rewards distributed to the quarry per year.
    pub annual_rewards_rate: u64,
    /// Rewards shared allocated to this quarry
    pub rewards_share: u64,

    /// Total number of tokens deposited into the quarry.
    pub total_tokens_deposited: u64,
    /// Number of [Miner]s.
    pub num_miners: u64,
}

/// An account that has staked tokens into a [Quarry].
#[account]
#[derive(Copy, Default, Debug)]
pub struct Miner {
    /// Key of the [Quarry] this [Miner] works on.
    pub quarry: Pubkey,
    /// Authority who manages this [Miner].
    /// All withdrawals of tokens must accrue to [TokenAccount]s owned by this account.
    pub authority: Pubkey,

    /// Bump.
    pub bump: u8,

    /// [TokenAccount] to hold the [Miner]'s staked LP tokens.
    pub token_vault_key: Pubkey,

    /// Stores the amount of tokens that the [Miner] may claim.
    /// Whenever the [Miner] claims tokens, this is reset to 0.
    pub rewards_earned: u64,

    /// A checkpoint of the [Quarry]'s reward tokens paid per staked token.
    ///
    /// When the [Miner] is initialized, this number starts at 0.
    /// On the first [quarry::stake_tokens], the [Quarry]#update_rewards_and_miner
    /// method is called, which updates this checkpoint to the current quarry value.
    ///
    /// On a [quarry::claim_rewards], the difference in checkpoints is used to calculate
    /// the amount of tokens owed.
    pub rewards_per_token_paid: u128,

    /// Number of tokens the [Miner] holds.
    pub balance: u64,

    /// Index of the [Miner].
    pub index: u64,
}

//! Calculates token distribution rates.

use crate::{Miner, Quarry};
use anchor_lang::prelude::*;
use spl_math::uint::U192;
use std::cmp;
use vipers::prelude::*;

/// Number of seconds in a year.
pub const SECONDS_PER_YEAR: u128 = 86_400 * 365;

/// Number of decimal points of precision that `rewards_per_token_stored` uses.
pub const PRECISION_MULTIPLIER: u128 = u64::MAX as u128;

/// Calculator for amount of tokens to pay out.
#[derive(Debug)]
pub struct Payroll {
    /// Timestamp of when rewards should end.
    pub famine_ts: i64,
    /// Timestamp of the last update.
    pub last_checkpoint_ts: i64,

    /// Amount of tokens to issue per year.
    pub annual_rewards_rate: u64,

    /// Amount of tokens to issue per staked token,
    /// multiplied by u64::MAX for precision.
    pub rewards_per_token_stored: u128,

    /// Total number of tokens deposited into the [Quarry].
    pub total_tokens_deposited: u64,
}

impl From<Quarry> for Payroll {
    /// Create a [Payroll] from a [Quarry].
    fn from(quarry: Quarry) -> Self {
        Self::new(
            quarry.famine_ts,
            quarry.last_update_ts,
            quarry.annual_rewards_rate,
            quarry.rewards_per_token_stored,
            quarry.total_tokens_deposited,
        )
    }
}

impl Payroll {
    /// Creates a new [Payroll].
    pub fn new(
        famine_ts: i64,
        last_checkpoint_ts: i64,
        annual_rewards_rate: u64,
        rewards_per_token_stored: u128,
        total_tokens_deposited: u64,
    ) -> Self {
        Self {
            famine_ts,
            last_checkpoint_ts,
            annual_rewards_rate,
            rewards_per_token_stored,
            total_tokens_deposited,
        }
    }

    /// Calculates the amount of rewards to pay out for each staked token.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L62
    fn calculate_reward_per_token_unsafe(&self, current_ts: i64) -> Option<u128> {
        if self.total_tokens_deposited == 0 {
            Some(self.rewards_per_token_stored)
        } else {
            let time_worked = self.compute_time_worked(current_ts)?;

            let reward = U192::from(time_worked)
                .checked_mul(PRECISION_MULTIPLIER.into())?
                .checked_mul(self.annual_rewards_rate.into())?
                .checked_div(SECONDS_PER_YEAR.into())?
                .checked_div(self.total_tokens_deposited.into())?;

            let precise_reward: u128 = reward.try_into().ok()?;

            self.rewards_per_token_stored.checked_add(precise_reward)
        }
    }

    /// Calculates the amount of rewards to pay for each staked token, performing safety checks.
    pub fn calculate_reward_per_token(&self, current_ts: i64) -> Result<u128> {
        invariant!(current_ts >= self.last_checkpoint_ts, InvalidTimestamp);
        Ok(unwrap_int!(
            self.calculate_reward_per_token_unsafe(current_ts)
        ))
    }

    /// Calculates the amount of rewards earned for the given number of staked tokens.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L72
    fn calculate_rewards_earned_unsafe(
        &self,
        current_ts: i64,
        tokens_deposited: u64,
        rewards_per_token_paid: u128,
        rewards_earned: u64,
    ) -> Option<u128> {
        let net_new_rewards = self
            .calculate_reward_per_token_unsafe(current_ts)?
            .checked_sub(rewards_per_token_paid)?;
        let rewards_earned = U192::from(tokens_deposited)
            .checked_mul(net_new_rewards.into())?
            .checked_div(PRECISION_MULTIPLIER.into())?
            .checked_add(rewards_earned.into())?;

        let precise_rewards_earned: u128 = rewards_earned.try_into().ok()?;
        Some(precise_rewards_earned)
    }

    /// Calculates the amount of rewards earned for the given number of staked tokens, with safety checks.
    /// <https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L72>
    pub fn calculate_rewards_earned(
        &self,
        current_ts: i64,
        tokens_deposited: u64,
        rewards_per_token_paid: u128,
        rewards_earned: u64,
    ) -> Result<u128> {
        invariant!(
            tokens_deposited <= self.total_tokens_deposited,
            NotEnoughTokens
        );
        invariant!(current_ts >= self.last_checkpoint_ts, InvalidTimestamp);
        let result = unwrap_int!(self.calculate_rewards_earned_unsafe(
            current_ts,
            tokens_deposited,
            rewards_per_token_paid,
            rewards_earned,
        ),);
        Ok(result)
    }

    fn calculate_claimable_upper_bound_unsafe(
        &self,
        current_ts: i64,
        rewards_per_token_paid: u128,
    ) -> Option<U192> {
        let time_worked = self.compute_time_worked(current_ts)?;

        let quarry_rewards_accrued = U192::from(time_worked)
            .checked_mul(self.annual_rewards_rate.into())?
            .checked_div(SECONDS_PER_YEAR.into())?;

        let net_rewards_per_token = self
            .rewards_per_token_stored
            .checked_sub(rewards_per_token_paid)?;
        let net_quarry_rewards = U192::from(net_rewards_per_token)
            .checked_mul(self.total_tokens_deposited.into())?
            .checked_div(PRECISION_MULTIPLIER.into())?;

        quarry_rewards_accrued.checked_add(net_quarry_rewards)
    }

    /// Sanity check on the amount of rewards to be claimed by the miner.
    pub fn sanity_check(
        &self,
        current_ts: i64,
        amount_claimable: u64,
        miner: &Miner,
    ) -> Result<()> {
        let rewards_upperbound =
            unwrap_int!(self
                .calculate_claimable_upper_bound_unsafe(current_ts, miner.rewards_per_token_paid,));
        let amount_claimable_less_already_earned =
            unwrap_int!(amount_claimable.checked_sub(miner.rewards_earned));

        if rewards_upperbound < amount_claimable_less_already_earned.into() {
            msg!(
                "current_ts: {}, rewards_upperbound: {}, amount_claimable: {}, payroll: {:?}, miner: {:?}",
                current_ts,
                rewards_upperbound,
                amount_claimable,
                self,
                miner,
            );
            invariant!(
                rewards_upperbound + 1 >= amount_claimable.into(), // Allow off by one.
                UpperboundExceeded
            );
        }

        Ok(())
    }

    /// Gets the latest time rewards were being distributed.
    pub fn last_time_reward_applicable(&self, current_ts: i64) -> i64 {
        cmp::min(current_ts, self.famine_ts)
    }

    /// Calculates the amount of seconds the [Payroll] should have applied rewards for.
    fn compute_time_worked(&self, current_ts: i64) -> Option<i64> {
        Some(cmp::max(
            0,
            self.last_time_reward_applicable(current_ts)
                .checked_sub(self.last_checkpoint_ts)?,
        ))
    }
}

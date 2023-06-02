//! Rewarder utilities.

use anchor_lang::prelude::*;
use num_traits::ToPrimitive;
use spl_math::uint::U192;
use vipers::prelude::*;

use crate::Rewarder;

impl Rewarder {
    /// Computes the amount of rewards a [crate::Quarry] should receive, annualized.
    /// This should be run only after `total_rewards_shares` has been set.
    /// Do not call this directly. Use `compute_quarry_annual_rewards_rate`.
    fn compute_quarry_annual_rewards_rate_unsafe(&self, quarry_rewards_share: u64) -> Option<u128> {
        let quarry_annual_rewards_rate = U192::from(self.annual_rewards_rate)
            .checked_mul(quarry_rewards_share.into())?
            .checked_div(self.total_rewards_shares.into())?;

        quarry_annual_rewards_rate.try_into().ok()
    }

    /// Computes the amount of rewards a [crate::Quarry] should receive, annualized.
    /// This should be run only after `total_rewards_shares` has been set.
    pub fn compute_quarry_annual_rewards_rate(&self, quarry_rewards_share: u64) -> Result<u64> {
        invariant!(
            quarry_rewards_share <= self.total_rewards_shares,
            InvalidRewardsShare
        );

        // no rewards if:
        if self.total_rewards_shares == 0 // no shares
            || self.annual_rewards_rate == 0 // rewards rate is zero
            || quarry_rewards_share == 0
        // quarry has no share
        {
            return Ok(0);
        }

        let raw_rate =
            unwrap_int!(self.compute_quarry_annual_rewards_rate_unsafe(quarry_rewards_share));
        Ok(unwrap_int!(raw_rate.to_u64()))
    }
}

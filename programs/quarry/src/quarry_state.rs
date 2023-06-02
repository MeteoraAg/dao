//! Quarry-related math and helpers.

use anchor_lang::prelude::*;
use vipers::prelude::*;

use crate::{payroll::Payroll, Miner, Quarry, Rewarder};
use num_traits::cast::ToPrimitive;

/// An action for a user to take on the staking pool.
pub enum StakeAction {
    /// Stake into a [Quarry].
    Stake,
    /// Withdraw from the [Quarry].
    Withdraw,
}

impl Quarry {
    /// Updates the quarry by synchronizing its rewards rate with the rewarder.
    pub fn update_rewards_internal(
        &mut self,
        current_ts: i64,
        rewarder: &Rewarder,
        payroll: &Payroll,
    ) -> Result<()> {
        let updated_rewards_per_token_stored = payroll.calculate_reward_per_token(current_ts)?;
        // Update quarry struct
        self.rewards_per_token_stored = updated_rewards_per_token_stored;
        self.annual_rewards_rate =
            rewarder.compute_quarry_annual_rewards_rate(self.rewards_share)?;
        self.last_update_ts = payroll.last_time_reward_applicable(current_ts);

        Ok(())
    }

    /// Updates the quarry and miner with the latest info.
    /// <https://github.com/Synthetixio/synthetix/blob/aeee6b2c82588681e1f99202663346098d1866ac/contracts/StakingRewards.sol#L158>
    pub fn update_rewards_and_miner(
        &mut self,
        miner: &mut Miner,
        rewarder: &Rewarder,
        current_ts: i64,
    ) -> Result<()> {
        let payroll: Payroll = (*self).into();
        self.update_rewards_internal(current_ts, rewarder, &payroll)?;

        let updated_rewards_earned = unwrap_int!(payroll
            .calculate_rewards_earned(
                current_ts,
                miner.balance,
                miner.rewards_per_token_paid,
                miner.rewards_earned,
            )?
            .to_u64());

        payroll.sanity_check(current_ts, updated_rewards_earned, miner)?;
        // Update miner struct
        miner.rewards_earned = updated_rewards_earned;
        miner.rewards_per_token_paid = self.rewards_per_token_stored;

        Ok(())
    }

    /// Processes a [StakeAction] for a [Miner],
    pub fn process_stake_action_internal(
        &mut self,
        action: StakeAction,
        current_ts: i64,
        rewarder: &Rewarder,
        miner: &mut Miner,
        amount: u64,
    ) -> Result<()> {
        self.update_rewards_and_miner(miner, rewarder, current_ts)?;
        match action {
            StakeAction::Stake => {
                miner.balance = unwrap_int!(miner.balance.checked_add(amount));
                self.total_tokens_deposited =
                    unwrap_int!(self.total_tokens_deposited.checked_add(amount));
            }
            StakeAction::Withdraw => {
                miner.balance = unwrap_int!(miner.balance.checked_sub(amount));
                self.total_tokens_deposited =
                    unwrap_int!(self.total_tokens_deposited.checked_sub(amount));
            }
        }

        Ok(())
    }
}

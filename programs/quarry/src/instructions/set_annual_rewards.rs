use crate::*;

/// Accounts for [quarry::set_annual_rewards].
#[derive(Accounts)]
pub struct SetAnnualRewards<'info> {
    /// [Rewarder],
    pub auth: MutableRewarderWithAuthority<'info>,
}

pub fn handler(ctx: Context<SetAnnualRewards>, new_rate: u64) -> Result<()> {
    invariant!(
        new_rate <= MAX_ANNUAL_REWARDS_RATE,
        MaxAnnualRewardsRateExceeded
    );
    let rewarder = &mut ctx.accounts.auth.rewarder;
    let previous_rate = rewarder.annual_rewards_rate;
    rewarder.annual_rewards_rate = new_rate;

    let current_ts = Clock::get()?.unix_timestamp;
    emit!(RewarderAnnualRewardsUpdateEvent {
        previous_rate,
        new_rate,
        timestamp: current_ts,
    });

    Ok(())
}
impl<'info> Validate<'info> for SetAnnualRewards<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.rewarder.assert_not_paused()?;
        self.auth.validate()?;
        Ok(())
    }
}
/// Emitted when the daily rewards rate is updated.
#[event]
pub struct RewarderAnnualRewardsUpdateEvent {
    /// Previous rate of rewards.
    pub previous_rate: u64,
    /// New rate of rewards.
    pub new_rate: u64,
    /// When the event took place.
    pub timestamp: i64,
}

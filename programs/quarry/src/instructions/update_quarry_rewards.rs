use crate::*;

/// Accounts for [quarry::update_quarry_rewards].
#[derive(Accounts)]
pub struct UpdateQuarryRewards<'info> {
    /// [Quarry].
    #[account(mut, has_one = rewarder)]
    pub quarry: Account<'info, Quarry>,

    /// [Rewarder].
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<UpdateQuarryRewards>) -> Result<()> {
    let current_ts = Clock::get()?.unix_timestamp;
    let rewarder = &ctx.accounts.rewarder;
    let payroll: Payroll = (*ctx.accounts.quarry).into();
    let quarry = &mut ctx.accounts.quarry;
    quarry.update_rewards_internal(current_ts, rewarder, &payroll)?;

    emit!(QuarryRewardsUpdateEvent {
        token_mint: quarry.token_mint_key,
        annual_rewards_rate: quarry.annual_rewards_rate,
        rewards_share: quarry.rewards_share,
        timestamp: current_ts,
    });

    Ok(())
}
impl<'info> Validate<'info> for UpdateQuarryRewards<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        Ok(())
    }
}
/// Emitted when a quarry's reward rate is updated.
#[event]
pub struct QuarryRewardsUpdateEvent {
    /// [Mint] of the [Quarry] token.
    pub token_mint: Pubkey,
    /// New annual rewards rate
    pub annual_rewards_rate: u64,
    /// New rewards share.
    pub rewards_share: u64,
    /// When the event took place.
    pub timestamp: i64,
}

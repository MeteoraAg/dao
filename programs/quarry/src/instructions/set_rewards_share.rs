use crate::*;

/// Accounts for [quarry::set_rewards_share].
#[derive(Accounts)]
pub struct SetRewardsShare<'info> {
    /// Authority of the rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.    
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,

    /// [Quarry] updated.
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,
}

pub fn handler(ctx: Context<SetRewardsShare>, new_share: u64) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    let quarry = &mut ctx.accounts.quarry;
    rewarder.total_rewards_shares = unwrap_int!(rewarder
        .total_rewards_shares
        .checked_add(new_share)
        .and_then(|v| v.checked_sub(quarry.rewards_share)));

    quarry.rewards_share = new_share;
    // Do not update annual_rewards_rate here. Just wait for the update_quarry_rewards call
    // because rewarder.total_rewards_shares may change with other quarry share changes
    // and quarry.annual_rewards_rate calculated here will become invalid.
    // The correct share update procedure is calling all set_rewards_share
    // first (with set_annual_rewards if needed) and update_quarry_rewards
    // for each quarry to finalize changes.
    // TODO: because update_quarry_rewards is permissionless better
    // to add protection from calling it after admin started to change the rates
    // and commit_changes instruction making possible to call update_quarry_rewards again

    Ok(())
}
impl<'info> Validate<'info> for SetRewardsShare<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);
        self.rewarder.assert_not_paused()?;

        invariant!(
            self.authority.key() == self.rewarder.admin
                || self.authority.key() == self.rewarder.mint_authority,
            Unauthorized
        );

        Ok(())
    }
}

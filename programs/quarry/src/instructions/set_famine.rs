use crate::*;

/// Accounts for [quarry::set_famine].
#[derive(Accounts)]
pub struct SetFamine<'info> {
    /// [Rewarder] of the [Quarry].
    pub auth: ReadOnlyRewarderWithAdmin<'info>,

    /// [Quarry] updated.
    #[account(mut, constraint = quarry.rewarder == auth.rewarder.key())]
    pub quarry: Account<'info, Quarry>,
}

/// Read-only [Rewarder] that requires the admin to be a signer.
#[derive(Accounts)]
pub struct ReadOnlyRewarderWithAdmin<'info> {
    /// Admin of the rewarder.
    pub admin: Signer<'info>,

    /// [Rewarder].
    #[account(has_one = admin)]
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<SetFamine>, famine_ts: i64) -> Result<()> {
    let quarry = &mut ctx.accounts.quarry;
    quarry.famine_ts = famine_ts;

    Ok(())
}

impl<'info> Validate<'info> for SetFamine<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.rewarder.assert_not_paused()?;
        self.auth.validate()?;
        Ok(())
    }
}
impl<'info> Validate<'info> for ReadOnlyRewarderWithAdmin<'info> {
    /// Validates the [crate::Rewarder] is correct.
    fn validate(&self) -> Result<()> {
        invariant!(self.admin.is_signer, Unauthorized);
        assert_keys_eq!(self.admin, self.rewarder.admin);
        Ok(())
    }
}

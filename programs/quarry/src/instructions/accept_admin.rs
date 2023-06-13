use crate::*;

/// Accounts for [quarry::accept_admin].
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    /// Admin of the next rewarder.
    pub admin: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<AcceptAdmin>) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    let next_admin = rewarder.pending_admin;
    rewarder.admin = next_admin;
    rewarder.pending_admin = Pubkey::default();
    Ok(())
}

impl<'info> Validate<'info> for AcceptAdmin<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        invariant!(
            self.rewarder.pending_admin != Pubkey::default(),
            PendingAuthorityNotSet
        );
        assert_keys_eq!(self.rewarder.pending_admin, self.admin, Unauthorized);
        Ok(())
    }
}

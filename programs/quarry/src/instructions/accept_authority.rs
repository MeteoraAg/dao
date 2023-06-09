use crate::*;

/// Accounts for [quarry::accept_authority].
#[derive(Accounts)]
pub struct AcceptAuthority<'info> {
    /// Authority of the next rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<AcceptAuthority>) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    let next_authority = rewarder.pending_authority;
    rewarder.authority = next_authority;
    rewarder.pending_authority = Pubkey::default();
    Ok(())
}

impl<'info> Validate<'info> for AcceptAuthority<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        invariant!(
            self.rewarder.pending_authority != Pubkey::default(),
            PendingAuthorityNotSet
        );
        assert_keys_eq!(
            self.rewarder.pending_authority,
            self.authority,
            Unauthorized
        );
        Ok(())
    }
}

use crate::*;

/// Accounts for [quarry::pause] and [quarry::unpause].
#[derive(Accounts)]
pub struct MutableRewarderWithPauseAuthority<'info> {
    /// Pause authority of the rewarder.
    pub pause_authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<MutableRewarderWithPauseAuthority>, is_paused: bool) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    rewarder.is_paused = is_paused;
    Ok(())
}

impl<'info> Validate<'info> for MutableRewarderWithPauseAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.pause_authority.is_signer, Unauthorized);

        invariant!(
            self.pause_authority.key() == self.rewarder.admin
                || self.pause_authority.key() == self.rewarder.pause_authority,
            Unauthorized
        );

        Ok(())
    }
}

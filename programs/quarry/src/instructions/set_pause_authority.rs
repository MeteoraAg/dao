use crate::*;
/// Accounts for [quarry::set_pause_authority].
#[derive(Accounts)]
pub struct SetPauseAuthority<'info> {
    /// [Rewarder].
    pub auth: MutableRewarderWithAuthority<'info>,

    /// The pause authority.
    /// CHECK: OK
    pub new_pause_authority: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<SetPauseAuthority>) -> Result<()> {
    let rewarder = &mut ctx.accounts.auth.rewarder;
    rewarder.pause_authority = ctx.accounts.new_pause_authority.key();
    Ok(())
}

impl<'info> Validate<'info> for SetPauseAuthority<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}

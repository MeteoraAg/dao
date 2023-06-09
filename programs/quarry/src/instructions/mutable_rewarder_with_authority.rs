use crate::ErrorCode::Unauthorized;
use crate::*;

/// Mutable [Rewarder] that requires the authority to be a signer.
#[derive(Accounts, Clone)]
pub struct MutableRewarderWithAuthority<'info> {
    /// Authority of the rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut, has_one = authority @ Unauthorized)]
    pub rewarder: Account<'info, Rewarder>,
}
pub fn handler_set_operator(
    ctx: Context<MutableRewarderWithAuthority>,
    operator: Pubkey,
) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    rewarder.operator = operator;
    Ok(())
}

impl<'info> Validate<'info> for MutableRewarderWithAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.rewarder.authority, self.authority, Unauthorized);
        Ok(())
    }
}

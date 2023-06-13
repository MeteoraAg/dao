use crate::ErrorCode::Unauthorized;
use crate::*;

/// Mutable [Rewarder] that requires the authority to be a signer.
#[derive(Accounts, Clone)]
pub struct MutableRewarderWithAuthority<'info> {
    /// Admin of the rewarder.
    pub admin: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut, has_one = admin @ Unauthorized)]
    pub rewarder: Account<'info, Rewarder>,
}
pub fn handler_set_mint_authority(
    ctx: Context<MutableRewarderWithAuthority>,
    mint_authority: Pubkey,
) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    rewarder.mint_authority = mint_authority;
    Ok(())
}

impl<'info> Validate<'info> for MutableRewarderWithAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.admin.is_signer, Unauthorized);
        assert_keys_eq!(self.rewarder.admin, self.admin, Unauthorized);
        Ok(())
    }
}

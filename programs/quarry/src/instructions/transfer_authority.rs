use crate::ErrorCode::Unauthorized;
use crate::*;

/// Accounts for [quarry::transfer_authority].
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    /// Authority of the rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut, has_one = authority @ Unauthorized)]
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<TransferAuthority>, new_authority: Pubkey) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    rewarder.pending_authority = new_authority;
    Ok(())
}
impl<'info> Validate<'info> for TransferAuthority<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        Ok(())
    }
}

use crate::ErrorCode::Unauthorized;
use crate::*;

/// Accounts for [quarry::transfer_authority].
#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    /// Admin of the rewarder.
    pub admin: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut, has_one = admin @ Unauthorized)]
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;
    rewarder.pending_admin = new_admin;
    Ok(())
}
impl<'info> Validate<'info> for TransferAdmin<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        Ok(())
    }
}

use crate::*;

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    /// The [MintWrapper].
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// The previous admin.
    pub admin: Signer<'info>,

    /// The next admin.
    /// CHECK: OK
    pub next_admin: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<TransferAdmin>) -> Result<()> {
    let mint_wrapper: &mut Account<MintWrapper> = &mut ctx.accounts.mint_wrapper;
    mint_wrapper.pending_admin = ctx.accounts.next_admin.key();

    emit!(MintWrapperAdminProposeEvent {
        mint_wrapper: mint_wrapper.key(),
        current_admin: mint_wrapper.admin,
        pending_admin: mint_wrapper.pending_admin,
    });
    Ok(())
}

impl<'info> Validate<'info> for TransferAdmin<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.admin, self.mint_wrapper.admin);
        assert_keys_neq!(self.next_admin, self.mint_wrapper.admin);

        Ok(())
    }
}
/// Emitted when a [MintWrapper]'s admin is proposed.
#[event]
pub struct MintWrapperAdminProposeEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,

    /// The [MintWrapper]'s current admin.
    pub current_admin: Pubkey,
    /// The [MintWrapper]'s pending admin.
    pub pending_admin: Pubkey,
}

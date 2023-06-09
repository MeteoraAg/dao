use crate::*;

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    /// The mint wrapper.
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// The new admin.
    pub pending_admin: Signer<'info>,
}

pub fn handler(ctx: Context<AcceptAdmin>) -> Result<()> {
    let mint_wrapper = &mut ctx.accounts.mint_wrapper;
    let previous_admin = mint_wrapper.admin;
    mint_wrapper.admin = ctx.accounts.pending_admin.key();
    mint_wrapper.pending_admin = Pubkey::default();

    emit!(MintWrapperAdminUpdateEvent {
        mint_wrapper: mint_wrapper.key(),
        previous_admin,
        admin: mint_wrapper.admin,
    });
    Ok(())
}

impl<'info> Validate<'info> for AcceptAdmin<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.pending_admin, self.mint_wrapper.pending_admin);
        Ok(())
    }
}

/// Emitted when a [MintWrapper]'s admin is transferred.
#[event]
pub struct MintWrapperAdminUpdateEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,

    /// The [MintWrapper]'s previous admin.
    pub previous_admin: Pubkey,
    /// The [MintWrapper]'s new admin.
    pub admin: Pubkey,
}

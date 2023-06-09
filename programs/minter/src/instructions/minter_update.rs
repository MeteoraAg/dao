use crate::*;
/// Updates a minter.
#[derive(Accounts)]
pub struct MinterUpdate<'info> {
    /// Owner of the [MintWrapper].
    pub auth: OnlyAdmin<'info>,
    /// Information about the minter.
    #[account(mut)]
    pub minter: Account<'info, Minter>,
}

pub fn handler(ctx: Context<MinterUpdate>, allowance: u64) -> Result<()> {
    let minter = &mut ctx.accounts.minter;
    let previous_allowance = minter.allowance;
    minter.allowance = allowance;

    let mint_wrapper = &mut ctx.accounts.auth.mint_wrapper;
    mint_wrapper.total_allowance = unwrap_int!(mint_wrapper
        .total_allowance
        .checked_add(allowance)
        .and_then(|v| v.checked_sub(previous_allowance)));

    emit!(MinterAllowanceUpdateEvent {
        mint_wrapper: minter.mint_wrapper,
        minter: minter.key(),
        previous_allowance,
        allowance: minter.allowance,
    });
    Ok(())
}
impl<'info> Validate<'info> for MinterUpdate<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        assert_keys_eq!(self.minter.mint_wrapper, self.auth.mint_wrapper);
        Ok(())
    }
}
/// Emitted when a [Minter]'s allowance is updated.
#[event]
pub struct MinterAllowanceUpdateEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,
    /// The [Minter].
    #[index]
    pub minter: Pubkey,

    /// The [Minter]'s previous allowance.
    pub previous_allowance: u64,
    /// The [Minter]'s new allowance.
    pub allowance: u64,
}

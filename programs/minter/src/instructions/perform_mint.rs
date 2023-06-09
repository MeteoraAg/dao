use crate::*;

/// Accounts for the perform_mint instruction.
#[derive(Accounts, Clone)]
pub struct PerformMint<'info> {
    /// [MintWrapper].
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// [Minter]'s authority.
    pub minter_authority: Signer<'info>,

    /// Token [Mint].
    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    /// Destination [TokenAccount] for minted tokens.
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    /// [Minter] information.
    #[account(mut)]
    pub minter: Account<'info, Minter>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<PerformMint>, amount: u64) -> Result<()> {
    let mint_wrapper = &ctx.accounts.mint_wrapper;
    let minter = &mut ctx.accounts.minter;
    invariant!(minter.allowance >= amount, MinterAllowanceExceeded);

    let new_supply = unwrap_int!(ctx.accounts.token_mint.supply.checked_add(amount));
    invariant!(new_supply <= mint_wrapper.hard_cap, HardcapExceeded);

    minter.allowance = unwrap_int!(minter.allowance.checked_sub(amount));
    minter.total_minted = unwrap_int!(minter.total_minted.checked_add(amount));

    let seeds = gen_wrapper_signer_seeds!(mint_wrapper);
    let proxy_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::MintTo {
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.destination.to_account_info(),
            authority: ctx.accounts.mint_wrapper.to_account_info(),
        },
        proxy_signer,
    );
    token::mint_to(cpi_ctx, amount)?;

    let mint_wrapper = &mut ctx.accounts.mint_wrapper;
    mint_wrapper.total_allowance = unwrap_int!(mint_wrapper.total_allowance.checked_sub(amount));
    mint_wrapper.total_minted = unwrap_int!(mint_wrapper.total_minted.checked_add(amount));

    // extra sanity checks
    ctx.accounts.token_mint.reload()?;
    invariant!(new_supply == ctx.accounts.token_mint.supply, Unauthorized);

    emit!(MinterMintEvent {
        mint_wrapper: mint_wrapper.key(),
        minter: minter.key(),
        amount,
        destination: ctx.accounts.destination.key(),
    });
    Ok(())
}

impl<'info> Validate<'info> for PerformMint<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(
            self.mint_wrapper.to_account_info().is_writable,
            Unauthorized
        );
        invariant!(self.minter.to_account_info().is_writable, Unauthorized);

        invariant!(self.minter_authority.is_signer, Unauthorized);
        invariant!(self.minter.allowance > 0, MinterAllowanceExceeded);
        assert_keys_eq!(self.minter.mint_wrapper, self.mint_wrapper);
        assert_keys_eq!(
            self.minter_authority,
            self.minter.minter_authority,
            Unauthorized
        );
        assert_keys_eq!(self.token_mint, self.mint_wrapper.token_mint);
        assert_keys_eq!(self.destination.mint, self.token_mint);
        Ok(())
    }
}
/// Emitted when a [Minter] performs a mint.
#[event]
pub struct MinterMintEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,
    /// The [Minter].
    #[index]
    pub minter: Pubkey,

    /// Amount minted.
    pub amount: u64,
    /// Mint destination.
    pub destination: Pubkey,
}

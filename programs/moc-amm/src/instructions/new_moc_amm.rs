use crate::*;

#[derive(Accounts)]
pub struct NewMocAmm<'info> {
    /// Base account.
    pub base: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"moc_amm".as_ref(),
            base.key().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MocAmm>(),
    )]
    pub moc_amm: Account<'info, MocAmm>,

    /// Token a fee account
    #[account(
        init,
        seeds = [b"token_a_fee".as_ref(), moc_amm.key().as_ref()],
        bump,
        payer = payer,
        token::mint = token_mint_a,
        token::authority = moc_amm,
    )]
    pub token_a_fee: Box<Account<'info, TokenAccount>>,
    /// Token mint account
    pub token_mint_a: Box<Account<'info, Mint>>,

    /// Token b fee account
    #[account(
        init,
        seeds = [b"token_b_fee".as_ref(), moc_amm.key().as_ref()],
        bump,
        payer = payer,
        token::mint = token_mint_b,
        token::authority = moc_amm,
    )]
    pub token_b_fee: Box<Account<'info, TokenAccount>>,
    /// Token mint account
    pub token_mint_b: Box<Account<'info, Mint>>,

    /// rent
    pub rent: Sysvar<'info, Rent>,

    /// token_program
    pub token_program: Program<'info, Token>,

    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

// fee is in bps
pub fn handler(ctx: Context<NewMocAmm>, fee: u64, lp_mint: Pubkey) -> Result<()> {
    let moc_amm = &mut ctx.accounts.moc_amm;
    moc_amm.lp_mint = lp_mint;
    moc_amm.fee = fee;
    moc_amm.token_a_fee = ctx.accounts.token_a_fee.key();
    moc_amm.token_b_fee = ctx.accounts.token_b_fee.key();
    moc_amm.token_a_mint = ctx.accounts.token_mint_a.key();
    moc_amm.token_b_mint = ctx.accounts.token_mint_b.key();
    moc_amm.bump = unwrap_bump!(ctx, "moc_amm");
    moc_amm.base = ctx.accounts.base.key();
    Ok(())
}

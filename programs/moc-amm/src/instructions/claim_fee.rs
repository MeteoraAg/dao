use crate::*;

#[derive(Accounts)]
pub struct ClaimFee<'info> {
    pub moc_amm: Account<'info, MocAmm>,
    /// token_program
    pub token_program: Program<'info, Token>,
    /// token_program
    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub dest_token_account: Box<Account<'info, TokenAccount>>,
}

pub fn handler(ctx: Context<ClaimFee>, amount: u64) -> Result<()> {
    let amm_state = &ctx.accounts.moc_amm;

    let pool_seeds = &[
        b"moc_amm".as_ref(),
        amm_state.base.as_ref(),
        &[amm_state.bump],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer {
                from: ctx.accounts.token_account.to_account_info(),
                to: ctx.accounts.dest_token_account.to_account_info(),
                authority: ctx.accounts.moc_amm.to_account_info(),
            },
            &[&pool_seeds[..]],
        ),
        amount,
    )?;

    Ok(())
}

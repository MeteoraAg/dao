use crate::*;

/// Accounts for [quarry::new_rewarder].
#[derive(Accounts)]
pub struct NewRewarder<'info> {
    /// Base. Arbitrary key.
    pub base: Signer<'info>,

    /// [Rewarder] of mines.
    #[account(
        init,
        seeds = [
            b"Rewarder".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Rewarder>()
    )]
    pub rewarder: Account<'info, Rewarder>,

    /// Initial admin of the rewarder.
    /// CHECK: OK
    pub admin: Signer<'info>,

    /// Payer of the [Rewarder] initialization.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,

    /// Mint wrapper.
    pub mint_wrapper: Account<'info, minter::MintWrapper>,

    /// Rewards token mint.
    pub rewards_token_mint: Account<'info, Mint>,
}

pub fn handler(ctx: Context<NewRewarder>) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;

    rewarder.base = ctx.accounts.base.key();
    rewarder.bump = unwrap_bump!(ctx, "rewarder");

    rewarder.admin = ctx.accounts.admin.key();
    rewarder.pending_admin = Pubkey::default();
    rewarder.mint_authority = ctx.accounts.admin.key();
    rewarder.pause_authority = ctx.accounts.admin.key();
    rewarder.is_paused = false;

    rewarder.annual_rewards_rate = 0;
    rewarder.num_quarries = 0;
    rewarder.total_rewards_shares = 0;
    rewarder.mint_wrapper = ctx.accounts.mint_wrapper.key();

    rewarder.rewards_token_mint = ctx.accounts.rewards_token_mint.key();

    let current_ts = Clock::get()?.unix_timestamp;
    emit!(NewRewarderEvent {
        admin: rewarder.admin,
        timestamp: current_ts,
    });

    Ok(())
}

impl<'info> Validate<'info> for NewRewarder<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.base.is_signer, Unauthorized);

        assert_keys_eq!(self.mint_wrapper.token_mint, self.rewards_token_mint);
        assert_keys_eq!(
            self.rewards_token_mint.mint_authority.unwrap(),
            self.mint_wrapper
        );

        Ok(())
    }
}
/// Emitted when a new [Rewarder] is created.
#[event]
pub struct NewRewarderEvent {
    /// Admin of the rewarder
    #[index]
    pub admin: Pubkey,
    /// When the event occurred.
    pub timestamp: i64,
}

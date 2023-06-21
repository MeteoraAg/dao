//! CreateBribe

use crate::*;
use crate::ErrorCode::MathOverflow;

/// Accounts for [gauge::create_bribe].
#[derive(Accounts)]
pub struct CreateBribe<'info> {
    /// The [Bribe] to be created.
    #[account(
            init,            
            space = 8 + std::mem::size_of::<Bribe>(),
            payer = payer
        )]
    pub bribe: Account<'info, Bribe>,

    /// System program.
    pub system_program: Program<'info, System>,

    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The [GaugeFactory].
    pub gauge_factory: Box<Account<'info, GaugeFactory>>,

    /// The [Gauge].
    #[account(mut, has_one = gauge_factory)]
    pub gauge: Box<Account<'info, Gauge>>,
    
    /// [TokenAccount] holding the token [Mint].
    #[account(
        init,
        seeds = [b"BribeVault".as_ref(), bribe.key().as_ref()],
        bump,
        payer = payer,
        token::mint = token_mint,
        token::authority = gauge_factory,
    )]
    pub token_account_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut, token::mint = token_mint)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub token_mint: Box<Account<'info, Mint>>,


    pub token_program: Program<'info, Token>,
}


pub fn get_total_bribe_rewards(reward_each_epoch: u64, bribe_epoch_end: u32, bribe_epoch_start: u32) -> Option<u64> {
    reward_each_epoch.checked_mul(u64::from(bribe_epoch_end.checked_add(1)?.checked_sub(bribe_epoch_start)?))
}

pub fn handler(ctx: Context<CreateBribe>, reward_each_epoch: u64, bribe_rewards_epoch_end: u32) -> Result<()> {
    // check bribe end is after voting epoch timestamp
    let gauge_factory = &ctx.accounts.gauge_factory;
    let current_voting_epoch = gauge_factory.current_voting_epoch;
    invariant!(
        bribe_rewards_epoch_end >= gauge_factory.current_voting_epoch,
        BribeEpochEndError
    );
    invariant!(
        reward_each_epoch > 0,
        BribeRewardsIsZero
    );

    // get total bribe rewards
    let total_bribe_rewards = get_total_bribe_rewards(reward_each_epoch, bribe_rewards_epoch_end, current_voting_epoch).ok_or(MathOverflow)?;
    // send to vault
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(),
            Transfer {
                from: ctx.accounts.token_account.to_account_info(),
                to: ctx.accounts.token_account_vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        ),
        total_bribe_rewards,
    )?;

    let bribe = &mut ctx.accounts.bribe;
    bribe.gauge = ctx.accounts.gauge.key();
    bribe.token_mint = ctx.accounts.token_mint.key();
    bribe.reward_each_epoch = reward_each_epoch;
    bribe.briber = ctx.accounts.payer.key();
    bribe.token_account_vault = ctx.accounts.token_account_vault.key();
    bribe.bribe_rewards_epoch_start = current_voting_epoch;
    bribe.bribe_rewards_epoch_end = bribe_rewards_epoch_end;

    emit!(BribeCreateEvent {
        gauge: ctx.accounts.gauge.key(),
        bribe: ctx.accounts.bribe.key(),
        bribe_rewards_epoch_start: current_voting_epoch,
        bribe_rewards_epoch_end,
        reward_each_epoch,
    });

    Ok(())
}

impl<'info> Validate<'info> for CreateBribe<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}


/// Event called in [gauge::create_bribe].
#[event]
pub struct BribeCreateEvent {
    #[index]
    /// The [Gauge].
    pub gauge: Pubkey,
    #[index]
    /// The Bribe.
    pub bribe: Pubkey,
    /// The Bribe epoch start.
    pub bribe_rewards_epoch_start: u32,
    /// The Bribe epoch end.
    pub bribe_rewards_epoch_end: u32,
    /// reward_each_epoch
    pub reward_each_epoch: u64,
}

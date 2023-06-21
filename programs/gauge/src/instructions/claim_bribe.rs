//! ClaimBribe
use crate::ErrorCode::MathOverflow;
use crate::*;

/// Accounts for [gauge::claim_bribe].
#[derive(Accounts)]
#[instruction(voting_epoch: u32)]
pub struct ClaimBribe<'info> {
    /// The [Bribe]
    #[account(mut, has_one = gauge, has_one = token_account_vault)]
    pub bribe: Account<'info, Bribe>,

    // ensure that gauge voter cannot claim again for an epoch with a bribe
    #[account(
        init,
        seeds = [b"EpochBribeVoter".as_ref(), voting_epoch.to_le_bytes().as_ref(), bribe.key().as_ref(), gauge_voter.key().as_ref()],
        bump,
        space = 8 + std::mem::size_of::<EpochBribeVoter>(),
        payer = vote_delegate,
    )]
    pub epoch_bribe_voter: Account<'info, EpochBribeVoter>,

    #[account(has_one = gauge_voter, constraint = epoch_gauge_voter.voting_epoch == voting_epoch)]
    pub epoch_gauge_voter: Box<Account<'info, EpochGaugeVoter>>,

    #[account(constraint = epoch_gauge_voter.voting_epoch == voting_epoch, has_one = gauge)]
    pub epoch_gauge: Box<Account<'info, EpochGauge>>,

    /// The [GaugeFactory].
    pub gauge_factory: Box<Account<'info, GaugeFactory>>,

    /// The [GaugeVoter].
    #[account(has_one = gauge_factory, has_one = escrow)]
    pub gauge_voter: Box<Account<'info, GaugeVoter>>,

    /// The [Gauge].
    #[account(has_one = gauge_factory)]
    pub gauge: Box<Account<'info, Gauge>>,

    #[account(mut)]
    pub token_account_vault: Box<Account<'info, TokenAccount>>,

    // token account to claim to
    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    /// The [Escrow] which owns this [EpochGaugeVote].
    #[account(has_one = vote_delegate @ crate::ErrorCode::UnauthorizedNotDelegate)]
    pub escrow: Box<Account<'info, voter::Escrow>>,

    /// The [Escrow::vote_delegate].
    #[account(mut)]
    pub vote_delegate: Signer<'info>,

    pub token_program: Program<'info, Token>,

    /// System program.
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimBribe>, voting_epoch: u32) -> Result<()> {
    let bribe = &mut ctx.accounts.bribe;
    let epoch_bribe_voter = &mut ctx.accounts.epoch_bribe_voter;
    invariant!(
        voting_epoch < ctx.accounts.gauge_factory.current_voting_epoch, // epoch is still voting
        CloseEpochNotElapsed
    );
    invariant!(
        voting_epoch >= bribe.bribe_rewards_epoch_start,
        BribeEpochEndError
    );
    invariant!(
        voting_epoch <= bribe.bribe_rewards_epoch_end,
        BribeEpochEndError
    );

    let rewards = bribe
        .get_rewards_for_an_epoch(
            ctx.accounts.epoch_gauge_voter.allocated_power,
            ctx.accounts.epoch_gauge.total_power,
        )
        .ok_or(MathOverflow)?;

    if rewards > 0 {
        let signer_seeds: &[&[&[u8]]] = gauge_factory_seeds!(ctx.accounts.gauge_factory);
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(),
                Transfer {
                    from: ctx.accounts.token_account_vault.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.gauge_factory.to_account_info(),
                },
                signer_seeds,
            ),
            rewards,
        )?;
        bribe.claimed_amount = bribe
            .claimed_amount
            .checked_add(rewards)
            .ok_or(MathOverflow)?;
    }

    epoch_bribe_voter.bribe = ctx.accounts.bribe.key();
    epoch_bribe_voter.gauge_voter = ctx.accounts.gauge_voter.key();
    epoch_bribe_voter.voting_epoch = voting_epoch;

    emit!(BribeClaimEvent {
        gauge: ctx.accounts.gauge.key(),
        bribe: ctx.accounts.bribe.key(),
        voting_epoch,
        rewards,
        token_account: ctx.accounts.token_account.key(),
        escrow: ctx.accounts.escrow.key(),
    });

    Ok(())
}

impl<'info> Validate<'info> for ClaimBribe<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Event called in [gauge::create_bribe].
#[event]
pub struct BribeClaimEvent {
    #[index]
    /// The [Gauge].
    pub gauge: Pubkey,
    #[index]
    /// The Bribe.
    pub bribe: Pubkey,
    /// The distribute rewards epoch.
    pub voting_epoch: u32,
    /// The Bribe epoch start.
    pub rewards: u64,
    /// The Bribe epoch end.
    pub token_account: Pubkey,
    /// The escrow.
    pub escrow: Pubkey,
}

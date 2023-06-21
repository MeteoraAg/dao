//! ClaimBribe
use crate::*;

/// Accounts for [gauge::claim_bribe].
#[derive(Accounts)]
#[instruction(voting_epoch: u32)]
pub struct ClawbackBribe<'info> {
    /// The [Bribe]
    #[account(has_one = gauge, has_one = token_account_vault, has_one = briber)]
    pub bribe: Account<'info, Bribe>,

    /// CHECK:
    #[account(
        seeds = [
            b"EpochGauge".as_ref(),
            gauge.key().as_ref(),
            voting_epoch.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub epoch_gauge: AccountInfo<'info>,

    /// The [GaugeFactory].
    pub gauge_factory: Box<Account<'info, GaugeFactory>>,

    /// The [Gauge].
    #[account(has_one = gauge_factory)]
    pub gauge: Box<Account<'info, Gauge>>,

    #[account(mut)]
    pub token_account_vault: Box<Account<'info, TokenAccount>>,

    // token account to claim to
    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub briber: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClawbackBribe>, voting_epoch: u32) -> Result<()> {
    let bribe = &ctx.accounts.bribe;
    // cannot clawback voting_epoch that is not finished
    invariant!(
        voting_epoch < ctx.accounts.gauge_factory.current_voting_epoch,
        ClawbackEpochIsNotCorrect
    );
    invariant!(
        voting_epoch >= bribe.bribe_rewards_epoch_start,
        ClawbackEpochIsNotCorrect
    );
    invariant!(
        voting_epoch <= bribe.bribe_rewards_epoch_end,
        ClawbackEpochIsNotCorrect
    );

    // check epoch_gauge is not existed or total_power is zero
    let mut data: &[u8] = &ctx.accounts.epoch_gauge.try_borrow_data()?;
    let reserve_state = EpochGauge::try_deserialize(&mut data);
    let can_clawback = if reserve_state.is_err() {
        // epoch gauge is not existed
        true
    } else {
        // epoch gauge is not voted
        let reserve_state = reserve_state?;
        reserve_state.total_power == 0
    };
    invariant!(can_clawback, EpochGaugeIsVoted);

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
        bribe.reward_each_epoch,
    )?;

    Ok(())
}

impl<'info> Validate<'info> for ClawbackBribe<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

//! Creates an [EpochGauge].

use crate::*;

/// Accounts for [gauge::create_epoch_gauge].
#[derive(Accounts)]
pub struct CreateEpochGauge<'info> {
    /// The [GaugeFactory].
    pub gauge_factory: Account<'info, GaugeFactory>,
    /// The [Gauge] to create an [EpochGauge] of.
    #[account(mut, has_one = amm_pool, has_one = gauge_factory)]
    pub gauge: Account<'info, Gauge>,

    /// The [EpochGauge] to be created.
    #[account(
        init,
        seeds = [
            b"EpochGauge".as_ref(),
            gauge.key().as_ref(),
            gauge_factory.voting_epoch()?.to_le_bytes().as_ref()
        ],
        bump,
        space = 8 + std::mem::size_of::<EpochGauge>(),
        payer = payer
    )]
    pub epoch_gauge: Account<'info, EpochGauge>,
    /// CHECK:
    pub amm_pool: UncheckedAccount<'info>,

    pub token_a_fee: Box<Account<'info, TokenAccount>>,

    pub token_b_fee: Box<Account<'info, TokenAccount>>,

    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateEpochGauge>) -> Result<()> {
    let epoch_gauge = &mut ctx.accounts.epoch_gauge;
    epoch_gauge.gauge = ctx.accounts.gauge.key();
    let voting_epoch = ctx.accounts.gauge_factory.voting_epoch()?;
    epoch_gauge.voting_epoch = voting_epoch;
    epoch_gauge.total_power = 0;

    let gauge = &mut ctx.accounts.gauge;

    let current_a_fee = ctx.accounts.token_a_fee.amount as u128;
    let current_b_fee = ctx.accounts.token_b_fee.amount as u128;

    epoch_gauge.token_a_fee = current_a_fee
        .checked_sub(
            gauge
                .cummulative_token_a_fee
                .checked_sub(gauge.cummulative_claimed_token_a_fee)
                .unwrap(),
        )
        .unwrap();
    epoch_gauge.token_b_fee = current_b_fee
        .checked_sub(
            gauge
                .cummulative_token_b_fee
                .checked_sub(gauge.cummulative_claimed_token_b_fee)
                .unwrap(),
        )
        .unwrap();

    gauge.cummulative_token_a_fee = gauge
        .cummulative_token_a_fee
        .checked_add(epoch_gauge.token_a_fee)
        .unwrap();
    gauge.cummulative_token_b_fee = gauge
        .cummulative_token_b_fee
        .checked_add(epoch_gauge.token_b_fee)
        .unwrap();

    emit!(EpochGaugeCreateEvent {
        gauge_factory: ctx.accounts.gauge.gauge_factory,
        quarry: ctx.accounts.gauge.quarry,
        token_a_fee: epoch_gauge.token_a_fee,
        token_b_fee: epoch_gauge.token_b_fee,
        voting_epoch,
    });

    Ok(())
}

impl<'info> Validate<'info> for CreateEpochGauge<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.token_a_fee.key(), self.gauge.token_a_fee_key);
        assert_keys_eq!(self.token_b_fee.key(), self.gauge.token_b_fee_key);

        Ok(())
    }
}

#[event]
/// Event called in called in [gauge::create_gauge_vote].
pub struct EpochGaugeCreateEvent {
    #[index]
    /// The [GaugeFactory].
    pub gauge_factory: Pubkey,
    #[index]
    /// The [quarry::Quarry] being voted on.
    pub quarry: Pubkey,
    /// The epoch associated with this [EpochGauge].
    pub voting_epoch: u32,
    /// Token a fee for this epoch
    pub token_a_fee: u128,
    /// Token b fee for this epoch
    pub token_b_fee: u128,
}

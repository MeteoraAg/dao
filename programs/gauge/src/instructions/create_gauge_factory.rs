//! Creates the [GaugeFactory].

use num_traits::ToPrimitive;
use vipers::prelude::*;

use crate::*;

/// Accounts for [gauge::create_gauge_factory].
#[derive(Accounts)]
pub struct CreateGaugeFactory<'info> {
    /// The [GaugeFactory] to be created.
    #[account(
        init,
        seeds = [
            b"GaugeFactory".as_ref(),
            base.key().as_ref(),
        ],
        bump,
        space = 8 + std::mem::size_of::<GaugeFactory>(),
        payer = payer
    )]
    pub gauge_factory: Account<'info, GaugeFactory>,

    /// Base.
    pub base: Signer<'info>,

    /// The Quarry [minter::Rewarder].
    /// CHECK: rewarder of token
    pub rewarder: UncheckedAccount<'info>,

    /// [voter::Locker] which determines gauge weights.
    pub locker: Account<'info, voter::Locker>,

    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateGaugeFactory>,
    foreman: Pubkey,
    epoch_duration_seconds: u32,
    first_epoch_starts_at: u64,
) -> Result<()> {
    let now = unwrap_int!(Clock::get()?.unix_timestamp.to_u64());
    invariant!(
        now <= first_epoch_starts_at,
        "first epoch must be in the future"
    );

    let gauge_factory = &mut ctx.accounts.gauge_factory;

    gauge_factory.base = ctx.accounts.base.key();
    gauge_factory.bump = *unwrap_int!(ctx.bumps.get("gauge_factory"));

    gauge_factory.rewarder = ctx.accounts.rewarder.key();
    // gaugemeister.operator = ctx.accounts.operator.key();
    gauge_factory.locker = ctx.accounts.locker.key();

    gauge_factory.foreman = foreman;
    gauge_factory.epoch_duration_seconds = epoch_duration_seconds;

    gauge_factory.current_voting_epoch = 1; //start from 1

    gauge_factory.next_epoch_starts_at = first_epoch_starts_at;

    emit!(CreateGaugeFactoryEvent {
        gauge_factory: gauge_factory.key(),
        rewarder: gauge_factory.rewarder,
        locker: ctx.accounts.locker.key(),
        first_rewards_epoch: first_epoch_starts_at,
        foreman,
    });

    Ok(())
}

impl<'info> Validate<'info> for CreateGaugeFactory<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Event called in [gauge::create_gauge_factory].
#[event]
pub struct CreateGaugeFactoryEvent {
    /// The [GaugeFactory] being created.
    #[index]
    pub gauge_factory: Pubkey,
    #[index]
    pub rewarder: Pubkey,
    /// Locker
    pub locker: Pubkey,
    /// Account which may enable/disable gauges on the [GaugeFactory].
    pub foreman: Pubkey,
    /// The first rewards epoch.
    pub first_rewards_epoch: u64,
}

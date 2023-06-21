//! Triggers the next epoch.

use num_traits::ToPrimitive;
use vipers::{invariant, unwrap_int};

use crate::*;

/// Accounts for [gauge::trigger_next_epoch].
#[derive(Accounts)]
pub struct TriggerNextEpoch<'info> {
    /// The [GaugeFactory].
    #[account(mut)]
    pub gauge_factory: Account<'info, GaugeFactory>,
}

pub fn handler(ctx: Context<TriggerNextEpoch>) -> Result<()> {
    let now = unwrap_int!(Clock::get()?.unix_timestamp.to_u64());
    msg!(
        "now: {}; next: {}",
        now,
        ctx.accounts.gauge_factory.next_epoch_starts_at
    );
    invariant!(
        now >= ctx.accounts.gauge_factory.next_epoch_starts_at,
        NextEpochNotReached
    );

    let gauge_factory = &mut ctx.accounts.gauge_factory;
    gauge_factory.current_voting_epoch = gauge_factory.distribute_rewards_epoch()?;
    gauge_factory.next_epoch_starts_at =
        unwrap_int!(now.checked_add(unwrap_int!(gauge_factory.epoch_duration_seconds.to_u64())));
    Ok(())
}

impl<'info> Validate<'info> for TriggerNextEpoch<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

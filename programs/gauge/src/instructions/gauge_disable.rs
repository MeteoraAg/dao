//! Disables a [Gauge].

use vipers::assert_keys_eq;

use crate::*;

/// Accounts for [gauge::gauge_disable].
#[derive(Accounts)]
pub struct GaugeDisable<'info> {
    /// The [GaugeFactory].
    pub gauge_factory: Account<'info, GaugeFactory>,
    /// The [Gauge] to disable.
    #[account(mut)]
    pub gauge: Account<'info, Gauge>,
    /// The [GaugeFactorty::foreman].
    pub foreman: Signer<'info>,
}

pub fn handler(ctx: Context<GaugeDisable>) -> Result<()> {
    let gauge = &mut ctx.accounts.gauge;
    gauge.is_disabled = true;
    emit!(GaugeDisableEvent {
        gauge_factory: ctx.accounts.gauge_factory.key(),
        gauge: gauge.key(),
        foreman: ctx.accounts.foreman.key(),
    });
    Ok(())
}

impl<'info> Validate<'info> for GaugeDisable<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.foreman,
            self.gauge_factory.foreman,
            UnauthorizedNotForeman
        );
        assert_keys_eq!(self.gauge.gauge_factory, self.gauge_factory);
        Ok(())
    }
}
/// Emitted on [gauge::gauge_disable].
#[event]
pub struct GaugeDisableEvent {
    /// The [Gauge].
    #[index]
    pub gauge: Pubkey,
    /// The [GaugeFactorty].
    #[index]
    pub gauge_factory: Pubkey,
    /// The [GaugeFactorty::foreman] that disabled the gauge.
    pub foreman: Pubkey,
}

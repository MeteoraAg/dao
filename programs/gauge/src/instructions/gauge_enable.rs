//! Enables a [Gauge].

use vipers::assert_keys_eq;

use crate::*;

/// Accounts for [gauge::gauge_enable].
#[derive(Accounts)]
pub struct GaugeEnable<'info> {
    /// The [GaugeFactory].
    pub gauge_factory: Account<'info, GaugeFactory>,
    /// The [Gauge] to enable.
    #[account(mut)]
    pub gauge: Account<'info, Gauge>,
    /// The [GaugeFactorty::foreman].
    pub foreman: Signer<'info>,
}

/// Emitted on [gauge::gauge_enable].
#[event]
pub struct GaugeEnableEvent {
    /// The [Gauge].
    #[index]
    pub gauge: Pubkey,
    /// The [GaugeFactorty].
    #[index]
    pub gauge_factory: Pubkey,
    /// The [GaugeFactorty::foreman] that enabled the gauge.
    pub foreman: Pubkey,
}

pub fn handler(ctx: Context<GaugeEnable>) -> Result<()> {
    let gauge = &mut ctx.accounts.gauge;
    gauge.is_disabled = false;
    emit!(GaugeEnableEvent {
        gauge_factory: ctx.accounts.gauge_factory.key(),
        gauge: gauge.key(),
        foreman: ctx.accounts.foreman.key(),
    });
    Ok(())
}

impl<'info> Validate<'info> for GaugeEnable<'info> {
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

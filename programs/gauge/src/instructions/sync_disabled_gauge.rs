//! Syncs a disabled [Gauge].

use crate::*;

/// Accounts for [gauge::sync_disabled_gauge].
#[derive(Accounts)]
pub struct SyncDisabledGauge<'info> {
    /// The [GaugeFactory].
    #[account(has_one = rewarder)]
    pub gauge_factory: Account<'info, GaugeFactory>,

    /// The [Gauge].
    pub gauge: Account<'info, Gauge>,

    /// [quarry::Quarry].
    #[account(mut)]
    pub quarry: Account<'info, quarry::Quarry>,

    /// [GaugeFactory::rewarder].
    /// CHECK: validated by key, not deserialized to save CU's.
    #[account(mut)]
    pub rewarder: UncheckedAccount<'info>,

    /// [quarry] program.
    pub quarry_program: Program<'info, quarry::program::Quarry>,
}

impl<'info> SyncDisabledGauge<'info> {
    fn disable_rewards(&self) -> Result<()> {
        // Only call CPI if the rewards share actually changed.
        if self.quarry.rewards_share != 0 {
            let signer_seeds: &[&[&[u8]]] = gauge_factory_seeds!(self.gauge_factory);
            quarry::cpi::set_rewards_share(
                CpiContext::new_with_signer(
                    self.quarry_program.to_account_info(),
                    quarry::cpi::accounts::SetRewardsShare {
                        authority: self.gauge_factory.to_account_info(), // gauge_factory is operator of rewarder
                        rewarder: self.rewarder.to_account_info(),
                        quarry: self.quarry.to_account_info(),
                    },
                    signer_seeds,
                ),
                0,
            )?;
        }

        Ok(())
    }
}

pub fn handler(ctx: Context<SyncDisabledGauge>) -> Result<()> {
    ctx.accounts.disable_rewards()
}

impl<'info> Validate<'info> for SyncDisabledGauge<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.gauge_factory, self.gauge.gauge_factory);
        assert_keys_eq!(self.gauge_factory.rewarder, self.rewarder);
        invariant!(self.gauge.is_disabled);
        assert_keys_eq!(self.quarry, self.gauge.quarry);
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        invariant!(
            self.gauge_factory.current_rewards_epoch != 0,
            GaugeEpochCannotBeZero
        );

        Ok(())
    }
}

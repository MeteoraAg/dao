//! Enables a [Gauge].

use crate::*;

/// Accounts for [gauge::sync_gauge].
#[derive(Accounts)]
pub struct SyncGauge<'info> {
    /// The [GaugeFactory].
    #[account(has_one = rewarder)]
    pub gauge_factory: Account<'info, GaugeFactory>,

    /// The [Gauge].
    pub gauge: Account<'info, Gauge>,

    /// The [EpochGauge].
    pub epoch_gauge: Account<'info, EpochGauge>,

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

impl<'info> SyncGauge<'info> {
    fn set_rewards_share(&self) -> Result<()> {
        // Only call CPI if the rewards share actually changed.
        if self.quarry.rewards_share != self.epoch_gauge.total_power {
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
                self.epoch_gauge.total_power,
            )?;
        }

        // Emit event showing the share update.
        emit!(SyncGaugeEvent {
            gauge_factory: self.gauge_factory.key(),
            gauge: self.epoch_gauge.gauge,
            epoch: self.epoch_gauge.voting_epoch,
            previous_share: self.quarry.rewards_share,
            new_share: self.epoch_gauge.total_power
        });

        Ok(())
    }
}

pub fn handler(ctx: Context<SyncGauge>) -> Result<()> {
    ctx.accounts.set_rewards_share()
}

impl<'info> Validate<'info> for SyncGauge<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.gauge_factory, self.gauge.gauge_factory);
        assert_keys_eq!(self.gauge_factory.rewarder, self.rewarder);
        assert_keys_eq!(self.epoch_gauge.gauge, self.gauge);

        assert_keys_eq!(self.quarry, self.gauge.quarry);
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        invariant!(
            self.gauge_factory.rewards_epoch()? != 0,
            GaugeEpochCannotBeZero
        );
        invariant!(
            self.epoch_gauge.voting_epoch == self.gauge_factory.rewards_epoch()?,
            GaugeWrongEpoch
        );

        Ok(())
    }
}

/// Emitted on [gauge::sync_gauge].
#[event]
pub struct SyncGaugeEvent {
    /// The [Gauge].
    #[index]
    pub gauge: Pubkey,
    /// The [GaugeFactory].
    #[index]
    pub gauge_factory: Pubkey,
    /// The epoch synced.
    #[index]
    pub epoch: u32,
    /// The previous [quarry::Quarry::rewards_share].
    pub previous_share: u64,
    /// The new [quarry::Quarry::rewards_share].
    pub new_share: u64,
}

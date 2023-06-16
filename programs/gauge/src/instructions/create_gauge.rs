//! Creates a [Gauge].

use crate::*;

/// Accounts for [gauge::create_gauge].
#[derive(Accounts)]
pub struct CreateGauge<'info> {
    /// The [Gauge] to be created.
    #[account(
        init,
        seeds = [
            b"Gauge".as_ref(),
            gauge_factory.key().as_ref(),
            quarry.key().as_ref(),
        ],
        bump,
        space = 8 + std::mem::size_of::<Gauge>(),
        payer = payer
    )]
    pub gauge: Account<'info, Gauge>,

    /// [GaugeFactory].
    pub gauge_factory: Account<'info, GaugeFactory>,

    /// [quarry::Quarry].
    #[account(has_one = amm_pool)]
    pub quarry: Account<'info, quarry::Quarry>,

    /// [amm::Amm].
    /// CHECK:
    pub amm_pool: UncheckedAccount<'info>,

    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateGauge>) -> Result<()> {
    let gauge = &mut ctx.accounts.gauge;
    gauge.gauge_factory = ctx.accounts.gauge_factory.key();
    gauge.quarry = ctx.accounts.quarry.key();

    // Since this is permissionless, gauges are disabled when they are created.
    gauge.is_disabled = true;

    gauge.amm_pool = ctx.accounts.quarry.amm_pool;
    // TODO handle the case when apmm update token a fee and token b fee account
    // Probably nothing we can do
    #[cfg(feature = "mainnet")]
    let amm_pool = { amm::AmmType::MeteoraAmm.get_amm(ctx.accounts.amm_pool.to_account_info())? };

    #[cfg(not(feature = "mainnet"))]
    let amm_pool = { amm::AmmType::MocAmm.get_amm(ctx.accounts.amm_pool.to_account_info())? };

    let (token_a_fee, token_b_fee) = amm_pool.get_fee_accounts();
    gauge.token_a_fee_key = token_a_fee;
    gauge.token_b_fee_key = token_b_fee;
    Ok(())
}

impl<'info> Validate<'info> for CreateGauge<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.gauge_factory.rewarder, self.quarry.rewarder);
        Ok(())
    }
}

/// Event called in [gauge::create_gauge].
#[event]
pub struct GaugeCreateEvent {
    #[index]
    /// The [GaugeFactory].
    pub gauge_factory: Pubkey,
    #[index]
    /// The Rewarder.
    pub rewarder: Pubkey,
    #[index]
    /// The [quarry::Quarry] being voted on.
    pub quarry: Pubkey,
    #[index]
    /// Owner of the Escrow of the [GaugeVoter].
    pub gauge_voter_owner: Pubkey,
}

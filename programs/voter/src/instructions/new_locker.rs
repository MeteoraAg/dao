use crate::*;

/// Accounts for [voter::new_locker].
#[derive(Accounts)]
pub struct NewLocker<'info> {
    /// Base.
    pub base: Signer<'info>,

    /// [Locker].
    #[account(
        init,
        seeds = [
            b"Locker".as_ref(),
            base.key().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Locker>()
    )]
    pub locker: Account<'info, Locker>,

    /// Mint of the token that can be used to join the [Locker].
    pub token_mint: Account<'info, Mint>,

    /// [Governor] associated with the [Locker].
    pub governor: Account<'info, Governor>,

    /// Payer of the initialization.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

impl<'info> NewLocker<'info> {
    /// Creates a new [Locker].
    pub fn new_locker(&mut self, bump: u8, expiration: i64, params: LockerParams) -> Result<()> {
        let locker = &mut self.locker;
        locker.token_mint = self.token_mint.key();
        locker.governor = self.governor.key();
        locker.base = self.base.key();
        locker.bump = bump;
        locker.params = params;
        locker.expiration = expiration;

        emit!(NewLockerEvent {
            governor: locker.governor,
            locker: locker.key(),
            token_mint: locker.token_mint,
            params,
        });

        Ok(())
    }
}

impl<'info> Validate<'info> for NewLocker<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

#[event]
/// Event called in [voter::new_locker].
pub struct NewLockerEvent {
    /// The governor for the [Locker].
    #[index]
    pub governor: Pubkey,
    /// The [Locker] being created.
    pub locker: Pubkey,
    /// Mint of the token that can be used to join the [Locker].
    pub token_mint: Pubkey,
    /// New [LockerParams].
    pub params: LockerParams,
}

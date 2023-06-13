use crate::*;

/// Staking accounts
///
/// This accounts struct is always used in the context of the user authority
/// staking into an account. This is NEVER used by an admin.
///
/// Validation should be extremely conservative.
#[derive(Accounts, Clone)]
pub struct UserStake<'info> {
    /// Miner authority (i.e. the user).
    pub authority: Signer<'info>,

    /// Miner.
    #[account(mut)]
    pub miner: Account<'info, Miner>,

    /// Quarry to claim from.
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,

    /// Vault of the miner.
    #[account(mut)]
    pub miner_vault: Account<'info, TokenAccount>,

    /// User's staked token account
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rewarder
    pub rewarder: Account<'info, Rewarder>,
}

pub fn handler_stake_tokens(ctx: Context<UserStake>, amount: u64) -> Result<()> {
    if amount == 0 {
        // noop
        return Ok(());
    }

    let quarry = &mut ctx.accounts.quarry;
    let clock = Clock::get()?;
    quarry.process_stake_action_internal(
        StakeAction::Stake,
        clock.unix_timestamp,
        &ctx.accounts.rewarder,
        &mut ctx.accounts.miner,
        amount,
    )?;

    let cpi_accounts = Transfer {
        from: ctx.accounts.token_account.to_account_info(),
        to: ctx.accounts.miner_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    // Transfer LP tokens to quarry vault
    token::transfer(cpi_context, amount)?;

    emit!(StakeEvent {
        timestamp: clock.unix_timestamp,
        authority: ctx.accounts.authority.key(),
        amount,
        token: ctx.accounts.token_account.mint,
    });
    Ok(())
}

pub fn handler_unstake_tokens(ctx: Context<UserStake>, amount: u64) -> Result<()> {
    if amount == 0 {
        // noop
        return Ok(());
    }
    invariant!(
        amount <= ctx.accounts.miner_vault.amount,
        InsufficientBalance
    );

    let clock = Clock::get()?;
    let quarry = &mut ctx.accounts.quarry;
    quarry.process_stake_action_internal(
        StakeAction::Withdraw,
        clock.unix_timestamp,
        &ctx.accounts.rewarder,
        &mut ctx.accounts.miner,
        amount,
    )?;

    // Sign a transfer instruction as the [Miner]
    let miner_seeds = &[
        b"Miner".as_ref(),
        ctx.accounts.miner.quarry.as_ref(),
        ctx.accounts.miner.authority.as_ref(),
        &[ctx.accounts.miner.bump],
    ];
    let signer_seeds = &[&miner_seeds[..]];
    let cpi_accounts = token::Transfer {
        from: ctx.accounts.miner_vault.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.miner.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    // Transfer out LP tokens from quarry vault
    token::transfer(cpi_ctx, amount)?;

    emit!(WithdrawEvent {
        timestamp: clock.unix_timestamp,
        authority: ctx.accounts.authority.key(),
        amount,
        token: ctx.accounts.token_account.mint,
    });
    Ok(())
}

impl<'info> Validate<'info> for UserStake<'info> {
    /// Validates the UserStake.
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;

        // authority
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.miner.authority);

        // quarry
        assert_keys_eq!(self.miner.quarry, self.quarry);

        // miner_vault
        let staked_mint = self.quarry.token_mint_key;
        assert_keys_eq!(self.miner.token_vault_key, self.miner_vault);
        assert_keys_eq!(self.miner_vault.mint, staked_mint);
        assert_keys_eq!(self.miner_vault.owner, self.miner);

        // token_account
        assert_keys_eq!(self.token_account.mint, staked_mint);

        // rewarder
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        Ok(())
    }
}
/// Emitted when tokens are staked into a [Quarry].
#[event]
pub struct StakeEvent {
    /// Authority staking.
    #[index]
    pub authority: Pubkey,
    /// Mint of token staked.
    #[index]
    pub token: Pubkey,
    /// Amount staked.
    pub amount: u64,
    /// When the event took place.
    pub timestamp: i64,
}

/// Emitted when tokens are withdrawn from a [Quarry].
#[event]
pub struct WithdrawEvent {
    /// Authority withdrawing.
    #[index]
    pub authority: Pubkey,
    /// Mint of token withdrawn.
    #[index]
    pub token: Pubkey,
    /// Amount withdrawn.
    pub amount: u64,
    /// When the event took place.
    pub timestamp: i64,
}

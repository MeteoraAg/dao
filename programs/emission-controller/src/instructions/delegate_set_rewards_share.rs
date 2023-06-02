use crate::*;

/// Accounts for [crate::emission_controller::delegate_set_rewards_share].
#[derive(Accounts)]
pub struct DelegateSetRewardsShare<'info> {
    /// Delegate accounts.
    pub with_delegate: WithDelegate<'info>,
    /// [Quarry].
    #[account(
        mut,
        constraint = quarry.rewarder == with_delegate.rewarder.key()
    )]
    pub quarry: Account<'info, quarry::Quarry>,
}

/// Accounts struct for instructions that must be signed by one of the delegates on the [Operator].
#[derive(Accounts, Clone)]
pub struct WithDelegate<'info> {
    /// The [Operator] of the [Rewarder].
    #[account(mut, has_one = rewarder)]
    pub operator: Account<'info, Operator>,
    /// The delegated account in one of the [Operator] roles.
    pub delegate: Signer<'info>,
    /// The [Rewarder].
    #[account(
        mut,
        constraint = rewarder.authority == operator.key() @ ErrorCode::OperatorNotRewarderAuthority
    )]
    pub rewarder: Account<'info, Rewarder>,
    /// Quarry mine
    pub quarry_mine_program: Program<'info, quarry::program::Quarry>,
}

impl<'info> WithDelegate<'info> {
    /// Creates the [quarry_mine::cpi::accounts::MutableRewarderWithAuthority] accounts.
    pub fn to_auth_accounts(&self) -> quarry::cpi::accounts::MutableRewarderWithAuthority<'info> {
        quarry::cpi::accounts::MutableRewarderWithAuthority {
            authority: self.operator.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
        }
    }

    /// Creates the [quarry_mine::cpi::accounts::MutableRewarderWithAuthority] accounts.
    pub fn to_readonly_auth_accounts(
        &self,
    ) -> quarry::cpi::accounts::ReadOnlyRewarderWithAuthority<'info> {
        quarry::cpi::accounts::ReadOnlyRewarderWithAuthority {
            authority: self.operator.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
        }
    }
}

impl<'info> Validate<'info> for DelegateSetRewardsShare<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.quarry.rewarder,
            self.with_delegate.rewarder,
            Unauthorized
        );
        assert_keys_eq!(
            self.with_delegate.operator.share_allocator,
            self.with_delegate.delegate,
            Unauthorized
        );
        self.with_delegate.validate()?;
        Ok(())
    }
}

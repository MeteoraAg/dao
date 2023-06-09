use crate::ErrorCode::Unauthorized;
use crate::*;
/// Only an admin is allowed to use instructions containing this struct.
#[derive(Accounts)]
pub struct OnlyAdmin<'info> {
    /// The [MintWrapper].
    #[account(mut, has_one = admin @ Unauthorized)]
    pub mint_wrapper: Account<'info, MintWrapper>,
    /// [MintWrapper::admin].
    pub admin: Signer<'info>,
}

impl<'info> Validate<'info> for OnlyAdmin<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.admin, self.mint_wrapper.admin);
        Ok(())
    }
}

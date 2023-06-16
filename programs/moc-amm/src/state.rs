//! State structs.

use crate::*;

// pub trait AmmPool {
//     fn get_fee_accounts(&self) -> (Pubkey, Pubkey);
//     fn get_lp_token_account(&self) -> Pubkey;
//     fn claim_fee<'a, 'b, 'c, 'info>(
//         &self,
//         token_account: &Account<'info, TokenAccount>,
//         dest_token_account: &Account<'info, TokenAccount>,
//         token_program: &Program<'info, Token>,
//         amm_pool: &UncheckedAccount<'info>,
//         amm_program: &UncheckedAccount<'info>,
//         amount: u64,
//     ) -> Result<()>;
// }

#[account]
#[derive(Default, Debug)]
/// State of pool account
pub struct MocAmm {
    /// base
    pub base: Pubkey,
    /// LP token mint of the pool
    pub lp_mint: Pubkey, //32
    /// Admin fee token account for token A. Used to receive trading fee.
    pub token_a_fee: Pubkey, //32
    /// Admin fee token account for token B. Used to receive trading fee.
    pub token_b_fee: Pubkey, //32

    /// Cached
    pub token_a_mint: Pubkey, //32
    /// Cached
    pub token_b_mint: Pubkey, //32

    /// Fee
    pub fee: u64,
    /// bump
    pub bump: u8,
}

// impl AmmPool for MocAmm {
//     fn get_fee_accounts(&self) -> (Pubkey, Pubkey) {
//         return (self.token_a_fee, self.token_b_fee);
//     }
//     fn get_lp_token_account(&self) -> Pubkey {
//         return self.lp_mint;
//     }

//     fn claim_fee<'a, 'b, 'c, 'info>(
//         &self,
//         token_account: &Account<'info, TokenAccount>,
//         dest_token_account: &Account<'info, TokenAccount>,
//         token_program: &Program<'info, Token>,
//         amm_pool: &UncheckedAccount<'info>,
//         amm_program: &UncheckedAccount<'info>,
//         amount: u64,
//     ) -> Result<()> {

//         let data: &[u8] = &amm_pool.try_borrow_data()?;
//         let amm_state = MocAmm::try_deserialize(&mut &*data)?;

//         let pool_seeds = &[
//             b"moc_amm".as_ref(),
//             amm_state.base.as_ref(),
//             &[amm_state.bump],
//         ];

//         token::transfer(
//             CpiContext::new_with_signer(
//                 token_program.to_account_info().clone(),
//                 Transfer {
//                     from: token_account.to_account_info(),
//                     to: dest_token_account.to_account_info(),
//                     authority: amm_pool.to_account_info(),
//                 },
//                 &[&pool_seeds[..]],
//             ),
//             amount,
//         )?;
//         Ok(())
//     }
// }

// /// AmmType struct
// #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
// pub enum AmmType {
//     /// Moc Amm
//     MocAmm,
//     /// Meteora Amm
//     MeteoraAmm,
// }

// impl AmmType {
//     /// get strategy handler
//     pub fn get_amm<'info>(&self, pool_account: AccountInfo<'info>) -> Result<Box<dyn AmmPool>> {
//         match self {
//             AmmType::MocAmm => {
//                 let data: &[u8] = &pool_account.try_borrow_data()?;
//                 Ok(Box::new(MocAmm::try_deserialize(&mut &*data)?))
//             }
//             AmmType::MeteoraAmm => panic!("implement me"),
//         }
//     }
// }

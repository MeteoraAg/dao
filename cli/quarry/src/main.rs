mod args;

use crate::args::*;
use anyhow::Ok;
use anyhow::Result;
use solana_program::system_program;
use utils_cli::token::get_or_create_ata;
// use utils_cli::*;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signer::keypair::*;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::{Client, Program};
use std::rc::Rc;
use std::str::FromStr;

use clap::*;

fn main() -> Result<()> {
    let opts = Opts::parse();
    let payer =
        read_keypair_file(opts.config_override.wallet_path).expect("Wallet keypair file not found");
    let wallet = payer.pubkey();

    println!("Wallet {:#?}", wallet);
    println!("Program ID: {:#?}", opts.config_override.program_id);

    let program_id = Pubkey::from_str(opts.config_override.program_id.as_str())?;
    let client = Client::new_with_options(
        opts.config_override.cluster,
        Rc::new(Keypair::from_bytes(&payer.to_bytes())?),
        CommitmentConfig::finalized(),
    );
    let base = match opts.config_override.base_path {
        Some(value) => {
            read_keypair_file(&*shellexpand::tilde(&value)).expect("Requires a keypair file")
        }
        None => Keypair::new(),
    };
    let program = client.program(program_id);
    match opts.command {
        CliCommand::NewRewarder { rewards_token_mint } => {
            new_rewarder(&program, rewards_token_mint, base)?;
        }
        CliCommand::SetGaugeFactoryMintAuthority {} => {
            set_gauge_factory_mint_authority(&program, base.pubkey())?;
        }
        CliCommand::SetPauseAuthority {
            new_pause_authority,
        } => {
            set_pause_authority(&program, new_pause_authority, base.pubkey())?;
        }
        CliCommand::Pause {} => {
            pause(&program, base.pubkey())?;
        }
        CliCommand::Unpause {} => {
            unpause(&program, base.pubkey())?;
        }
        CliCommand::TransferAdmin { new_admin } => {
            transfer_admin(&program, new_admin, base.pubkey())?;
        }
        CliCommand::AcceptAdmin {} => {
            accept_admin(&program, base.pubkey())?;
        }
        CliCommand::SetAnnualRewards { new_rate } => {
            set_annual_rewards(&program, new_rate, base.pubkey())?;
        }
        CliCommand::CreateQuarry { token_mint } => {
            create_quarry(&program, token_mint, base.pubkey())?;
        }
        CliCommand::SetRewardsShare {
            new_share,
            token_mint,
        } => {
            set_rewards_share(&program, new_share, token_mint, base.pubkey())?;
        }
        CliCommand::SetFamine {
            famine_ts,
            token_mint,
        } => {
            set_famine_ts(&program, famine_ts, token_mint, base.pubkey())?;
        }
        CliCommand::UpdateQuarryRewards { token_mint } => {
            update_quarry_rewards(&program, token_mint, base.pubkey())?;
        }
        CliCommand::CreateMiner { token_mint } => {
            create_miner(&program, token_mint, base.pubkey())?;
        }
        CliCommand::ClaimRewards { token_mint } => {
            claim_rewards(&program, token_mint, base.pubkey())?;
        }
        CliCommand::StakeToken { token_mint, amount } => {
            stake_token(&program, amount, token_mint, base.pubkey())?;
        }
        CliCommand::UnstakeToken { token_mint, amount } => {
            unstake_token(&program, amount, token_mint, base.pubkey())?;
        }
    }

    Ok(())
}
fn new_rewarder(program: &Program, rewards_token_mint: Pubkey, base_kp: Keypair) -> Result<()> {
    let base = base_kp.pubkey();
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::NewRewarder {
            base,
            rewarder,
            admin: program.payer(),
            payer: program.payer(),
            system_program: system_program::id(),

            rewards_token_mint,
            mint_wrapper,
        })
        .args(quarry::instruction::NewRewarder {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn set_gauge_factory_mint_authority(program: &Program, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::MutableRewarderWithAuthority {
            admin: program.payer(),
            rewarder,
        })
        .args(quarry::instruction::SetMintAuthority {
            mint_authority: gauge_factory,
        });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn set_pause_authority(program: &Program, new_pause_authority: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::SetPauseAuthority {
            auth: quarry::accounts::MutableRewarderWithAuthority {
                admin: program.payer(),
                rewarder,
            },
            new_pause_authority,
        })
        .args(quarry::instruction::SetPauseAuthority {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn pause(program: &Program, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::MutableRewarderWithPauseAuthority {
            pause_authority: program.payer(),
            rewarder,
        })
        .args(quarry::instruction::Pause {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn unpause(program: &Program, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::MutableRewarderWithPauseAuthority {
            pause_authority: program.payer(),
            rewarder,
        })
        .args(quarry::instruction::Unpause {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn transfer_admin(program: &Program, new_admin: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::TransferAdmin {
            admin: program.payer(),
            rewarder,
        })
        .args(quarry::instruction::TransferAdmin { new_admin });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn accept_admin(program: &Program, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::AcceptAdmin {
            admin: program.payer(),
            rewarder,
        })
        .args(quarry::instruction::AcceptAdmin {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn set_annual_rewards(program: &Program, new_rate: u64, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::SetAnnualRewards {
            auth: quarry::accounts::MutableRewarderWithAuthority {
                admin: program.payer(),
                rewarder,
            },
        })
        .args(quarry::instruction::SetAnnualRewards { new_rate });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}
fn create_quarry(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let builder = program
        .request()
        .accounts(quarry::accounts::CreateQuarry {
            auth: quarry::accounts::MutableRewarderWithAuthority {
                admin: program.payer(),
                rewarder,
            },
            token_mint,
            quarry,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(quarry::instruction::CreateQuarry {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn set_rewards_share(
    program: &Program,
    new_share: u64,
    token_mint: Pubkey,
    base: Pubkey,
) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let builder = program
        .request()
        .accounts(quarry::accounts::SetRewardsShare {
            authority: program.payer(),
            quarry,
            rewarder,
        })
        .args(quarry::instruction::SetRewardsShare { new_share });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn set_famine_ts(
    program: &Program,
    famine_ts: i64,
    token_mint: Pubkey,
    base: Pubkey,
) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let builder = program
        .request()
        .accounts(quarry::accounts::SetFamine {
            auth: quarry::accounts::ReadOnlyRewarderWithAdmin {
                admin: program.payer(),
                rewarder,
            },
            quarry,
        })
        .args(quarry::instruction::SetFamine { famine_ts });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn update_quarry_rewards(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let builder = program
        .request()
        .accounts(quarry::accounts::UpdateQuarryRewards { rewarder, quarry })
        .args(quarry::instruction::UpdateQuarryRewards {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn create_miner(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (miner, _bump) = Pubkey::find_program_address(
        &[b"Miner".as_ref(), quarry.as_ref(), program.payer().as_ref()],
        &quarry::id(),
    );
    let (miner_vault, _bump) =
        Pubkey::find_program_address(&[b"MinerVault".as_ref(), miner.as_ref()], &quarry::id());
    let builder = program
        .request()
        .accounts(quarry::accounts::CreateMiner {
            authority: program.payer(),
            miner,
            miner_vault,
            rewarder,
            quarry,
            payer: program.payer(),
            token_mint,
            system_program: system_program::id(),
            token_program: spl_token::id(),
        })
        .args(quarry::instruction::CreateMiner {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn claim_rewards(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let rewarder_state: quarry::Rewarder = program.account(rewarder)?;

    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());
    let (minter, _bump) = Pubkey::find_program_address(
        &[
            b"MintWrapperMinter".as_ref(),
            mint_wrapper.as_ref(),
            rewarder.as_ref(),
        ],
        &minter::id(),
    );

    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (miner, _bump) = Pubkey::find_program_address(
        &[b"Miner".as_ref(), quarry.as_ref(), program.payer().as_ref()],
        &quarry::id(),
    );

    let rewards_token_account =
        get_or_create_ata(program, rewarder_state.rewards_token_mint, program.payer())?;
    let builder = program
        .request()
        .accounts(quarry::accounts::ClaimRewards {
            mint_wrapper,
            mint_wrapper_program: minter::id(),
            minter,
            rewards_token_mint: rewarder_state.rewards_token_mint,
            rewards_token_account,
            claim: quarry::accounts::UserClaim {
                authority: program.payer(),
                miner,
                quarry,
                token_program: spl_token::id(),
                rewarder,
            },
        })
        .args(quarry::instruction::ClaimRewards {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn stake_token(program: &Program, amount: u64, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());

    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (miner, _bump) = Pubkey::find_program_address(
        &[b"Miner".as_ref(), quarry.as_ref(), program.payer().as_ref()],
        &quarry::id(),
    );
    let (miner_vault, _bump) =
        Pubkey::find_program_address(&[b"MinerVault".as_ref(), miner.as_ref()], &quarry::id());

    let token_account = get_or_create_ata(program, token_mint, program.payer())?;
    let builder = program
        .request()
        .accounts(quarry::accounts::UserStake {
            authority: program.payer(),
            miner,
            quarry,
            miner_vault,
            token_account,
            rewarder,
            token_program: spl_token::id(),
        })
        .args(quarry::instruction::StakeTokens { amount });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn unstake_token(program: &Program, amount: u64, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());

    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (miner, _bump) = Pubkey::find_program_address(
        &[b"Miner".as_ref(), quarry.as_ref(), program.payer().as_ref()],
        &quarry::id(),
    );
    let (miner_vault, _bump) =
        Pubkey::find_program_address(&[b"MinerVault".as_ref(), miner.as_ref()], &quarry::id());

    let token_account = get_or_create_ata(program, token_mint, program.payer())?;
    let builder = program
        .request()
        .accounts(quarry::accounts::UserStake {
            authority: program.payer(),
            miner,
            quarry,
            miner_vault,
            token_account,
            rewarder,
            token_program: spl_token::id(),
        })
        .args(quarry::instruction::UnstakeTokens { amount });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

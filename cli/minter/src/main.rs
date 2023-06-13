mod args;

use crate::args::*;
use anyhow::Ok;
use anyhow::Result;
use solana_program::system_program;
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
        CliCommand::SetMintAthority { token_mint } => {
            set_mint_authority(&program, token_mint, base.pubkey())?;
        }
        CliCommand::NewMintWrapper {
            hard_cap,
            token_mint,
        } => {
            new_mint_wrapper(&program, hard_cap, token_mint, base)?;
        }
        CliCommand::NewMinter { minter_authority } => {
            new_minter(&program, minter_authority, base.pubkey())?;
        }
        CliCommand::TransferAdmin { next_admin } => {
            transfer_admin(&program, next_admin, base.pubkey())?;
        }
        CliCommand::AcceptAdmin {} => {
            accept_admin(&program, base.pubkey())?;
        }
        CliCommand::SetAllowance {
            allowance,
            minter_authority,
        } => {
            set_allowance(&program, allowance, minter_authority, base.pubkey())?;
        }
    }

    Ok(())
}

fn set_mint_authority(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());

    let instructions = vec![spl_token::instruction::set_authority(
        &spl_token::id(),
        &token_mint,
        Some(&mint_wrapper),
        spl_token::instruction::AuthorityType::MintTokens,
        &program.payer(),
        &[],
    )?];

    let builder = program.request();
    let builder = instructions
        .into_iter()
        .fold(builder, |bld, ix| bld.instruction(ix));

    // let result = simulate_transaction(&builder, program, &vec![&default_keypair()]).unwrap();
    // println!("{:?}", result);
    // return Ok(());

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn new_mint_wrapper(
    program: &Program,
    hard_cap: u64,
    token_mint: Pubkey,
    base_kp: Keypair,
) -> Result<()> {
    let base = base_kp.pubkey();
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());

    println!("new mint wrapper {:?}", mint_wrapper);

    let builder = program
        .request()
        .accounts(minter::accounts::NewWrapper {
            base,
            mint_wrapper,
            token_mint,
            admin: program.payer(),
            payer: program.payer(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
        })
        .args(minter::instruction::NewWrapper { hard_cap })
        .signer(&base_kp);
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn new_minter(program: &Program, minter_authority: Pubkey, base: Pubkey) -> Result<()> {
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());
    let (minter, _bump) = Pubkey::find_program_address(
        &[
            b"MintWrapperMinter".as_ref(),
            mint_wrapper.as_ref(),
            minter_authority.as_ref(),
        ],
        &minter::id(),
    );
    let builder = program
        .request()
        .accounts(minter::accounts::NewMinter {
            auth: minter::accounts::OnlyAdmin {
                mint_wrapper,
                admin: program.payer(),
            },
            minter_authority,
            minter,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(minter::instruction::NewMinter {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn transfer_admin(program: &Program, next_admin: Pubkey, base: Pubkey) -> Result<()> {
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());

    let builder = program
        .request()
        .accounts(minter::accounts::TransferAdmin {
            mint_wrapper,
            next_admin,
            admin: program.payer(),
        })
        .args(minter::instruction::TransferAdmin {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn accept_admin(program: &Program, base: Pubkey) -> Result<()> {
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());

    let builder = program
        .request()
        .accounts(minter::accounts::AcceptAdmin {
            mint_wrapper,
            pending_admin: program.payer(),
        })
        .args(minter::instruction::AcceptAdmin {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn set_allowance(
    program: &Program,
    allowance: u64,
    minter_authority: Pubkey,
    base: Pubkey,
) -> Result<()> {
    let (mint_wrapper, bump) =
        Pubkey::find_program_address(&[b"MintWrapper".as_ref(), base.as_ref()], &minter::id());
    let (minter, _bump) = Pubkey::find_program_address(
        &[
            b"MintWrapperMinter".as_ref(),
            mint_wrapper.as_ref(),
            minter_authority.as_ref(),
        ],
        &minter::id(),
    );

    let builder = program
        .request()
        .accounts(minter::accounts::MinterUpdate {
            auth: minter::accounts::OnlyAdmin {
                mint_wrapper,
                admin: program.payer(),
            },
            minter,
        })
        .args(minter::instruction::MinterUpdate { allowance });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

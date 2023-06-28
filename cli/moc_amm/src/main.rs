mod args;
use crate::args::*;
use anyhow::Result;
use utils_cli::token::create_token_mint;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signer::keypair::*;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::{Client, Program};
use anchor_lang::solana_program::sysvar;
use clap::*;
use std::rc::Rc;
use std::str::FromStr;

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
        CliCommand::NewAmm { lp_mint } => {
            new_amm(&program, base, lp_mint)?;
        }
    }

    Ok(())
}

fn new_amm(program: &Program, base_keypair: Keypair, lp_mint: Pubkey) -> Result<()> {
    let base = base_keypair.pubkey();
    let (moc_amm, bump) =
        Pubkey::find_program_address(&[b"moc_amm".as_ref(), base.as_ref()], &moc_amm::id());

    // create a mint
    let token_mint_a_kp = Keypair::new();
    create_token_mint(program, &token_mint_a_kp)?;
    let token_mint_a = token_mint_a_kp.pubkey();

    let (token_a_fee, bump) =
        Pubkey::find_program_address(&[b"token_a_fee".as_ref(), moc_amm.as_ref()], &moc_amm::id());

    let token_mint_b_kp = Keypair::new();
    create_token_mint(program, &token_mint_b_kp)?;
    let token_mint_b = token_mint_b_kp.pubkey();
    let (token_b_fee, bump) =
        Pubkey::find_program_address(&[b"token_b_fee".as_ref(), moc_amm.as_ref()], &moc_amm::id());

    let builder = program
        .request()
        .accounts(moc_amm::accounts::NewMocAmm {
            moc_amm,
            base,
            token_mint_a,
            token_mint_b,
            token_a_fee,
            token_b_fee,
            rent: sysvar::rent::ID,
            token_program: spl_token::ID,
            payer: program.payer(),
            system_program: solana_program::system_program::ID,
        })
        .args(moc_amm::instruction::NewMocAmm { fee: 30, lp_mint })
        .signer(&base_keypair);
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

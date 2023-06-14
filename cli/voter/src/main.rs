mod args;
use crate::args::*;
use anyhow::Result;
use utils_cli::*;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signer::keypair::*;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::{Client, Program};
use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use anchor_spl::associated_token::get_associated_token_address;
use clap::*;
use solana_program::instruction::Instruction;
use std::rc::Rc;
use std::str::FromStr;
use utils_cli::token::get_or_create_ata;

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
        CliCommand::NewLocker {
            token_mint,
            expiration,
            max_stake_vote_multiplier,
            min_stake_duration,
            max_stake_duration,
            proposal_activation_min_votes,
        } => {
            new_locker(
                &program,
                base,
                token_mint,
                expiration,
                max_stake_vote_multiplier,
                min_stake_duration,
                max_stake_duration,
                proposal_activation_min_votes,
            )?;
        }
        CliCommand::NewEscrow { base } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            new_escrow(&program, locker)?;
        }
        CliCommand::IncreaseLockedAmount { base, amount } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            increase_locked_amount(&program, locker, amount)?;
        }
        CliCommand::ExtendLockDuration { base, duration } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            extend_locked_duration(&program, locker, duration)?;
        }
        CliCommand::Withdraw { base } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            withdraw(&program, locker)?;
        }
        CliCommand::ActivateProposal { base, proposal } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            active_proposal(&program, locker, proposal)?;
        }
        CliCommand::CastVote {
            base,
            proposal,
            side,
        } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            cast_vote(&program, locker, proposal, side)?;
        }
        CliCommand::SetVoteDelegate { base, new_delegate } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            set_vote_delegate(&program, locker, new_delegate)?;
        }
        CliCommand::ViewLocker { base } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            let locker: voter::Locker = program.account(locker)?;
            println!("{:?}", locker);
        }
        CliCommand::ViewEscrow { base, owner } => {
            let (locker, _bump) =
                Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
            let (escrow, _bump) = Pubkey::find_program_address(
                &[b"Escrow".as_ref(), locker.as_ref(), owner.as_ref()],
                &voter::id(),
            );
            let escrow_state: voter::Escrow = program.account(escrow)?;
            println!("{:?}", escrow_state);
        }
    }

    Ok(())
}

fn new_locker(
    program: &Program,
    base_keypair: Keypair,
    token_mint: Pubkey,
    expiration: i64,
    max_stake_vote_multiplier: u8,
    min_stake_duration: u64,
    max_stake_duration: u64,
    proposal_activation_min_votes: u64,
) -> Result<()> {
    let base = base_keypair.pubkey();
    let (governor, bump) =
        Pubkey::find_program_address(&[b"MeteoraGovernor".as_ref(), base.as_ref()], &govern::id());

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    println!("locker address {}", locker);

    let builder = program
        .request()
        .accounts(voter::accounts::NewLocker {
            base,
            locker,
            token_mint,
            governor,
            payer: program.payer(),
            system_program: solana_program::system_program::ID,
        })
        .args(voter::instruction::NewLocker {
            expiration,
            params: voter::LockerParams {
                max_stake_vote_multiplier,
                min_stake_duration,
                max_stake_duration,
                proposal_activation_min_votes,
            },
        })
        .signer(&base_keypair);
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn new_escrow(program: &Program, locker: Pubkey) -> Result<()> {
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let builder = program
        .request()
        .accounts(voter::accounts::NewEscrow {
            locker,
            escrow,
            escrow_owner: program.payer(),
            payer: program.payer(),
            system_program: solana_program::system_program::ID,
        })
        .args(voter::instruction::NewEscrow {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn increase_locked_amount(program: &Program, locker: Pubkey, amount: u64) -> Result<()> {
    let locker_state: voter::Locker = program.account(locker)?;
    println!("{:?}", locker_state);
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let mut instructions = vec![];
    let escrow_info = program.rpc().get_account(&escrow);
    if escrow_info.is_err() {
        instructions = vec![Instruction {
            accounts: voter::accounts::NewEscrow {
                locker,
                escrow,
                escrow_owner: program.payer(),
                payer: program.payer(),
                system_program: solana_program::system_program::ID,
            }
            .to_account_metas(None),
            data: voter::instruction::NewEscrow {}.data(),
            program_id: voter::id(),
        }];
    }
    let escrow_tokens = get_or_create_ata(program, locker_state.token_mint, escrow)?;
    let source_tokens = get_or_create_ata(program, locker_state.token_mint, program.payer())?;
    instructions.push(Instruction {
        accounts: voter::accounts::IncreaseLockedAmount {
            locker,
            escrow,
            escrow_tokens,
            source_tokens,
            payer: program.payer(),
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        data: voter::instruction::IncreaseLockedAmount { amount }.data(),
        program_id: voter::id(),
    });

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

fn extend_locked_duration(program: &Program, locker: Pubkey, duration: i64) -> Result<()> {
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let builder = program
        .request()
        .accounts(voter::accounts::ExtendLockDuration {
            locker,
            escrow,
            escrow_owner: program.payer(),
        })
        .args(voter::instruction::ExtendLockDuration { duration });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn withdraw(program: &Program, locker: Pubkey) -> Result<()> {
    let locker_state: voter::Locker = program.account(locker)?;
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );
    let escrow_tokens = get_associated_token_address(&escrow, &locker_state.token_mint);

    let destination_tokens =
        get_associated_token_address(&program.payer(), &locker_state.token_mint);

    let builder = program
        .request()
        .accounts(voter::accounts::Withdraw {
            locker,
            escrow,
            escrow_tokens,
            destination_tokens,
            escrow_owner: program.payer(),
            payer: program.payer(),
            token_program: anchor_spl::token::ID,
        })
        .args(voter::instruction::Withdraw {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn active_proposal(program: &Program, locker: Pubkey, proposal: Pubkey) -> Result<()> {
    let locker_state: voter::Locker = program.account(locker)?;
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let builder = program
        .request()
        .accounts(voter::accounts::ActivateProposal {
            locker,
            escrow,
            proposal,
            escrow_owner: program.payer(),
            governor: locker_state.governor,
            govern_program: govern::ID,
        })
        .args(voter::instruction::ActivateProposal {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn cast_vote(program: &Program, locker: Pubkey, proposal: Pubkey, side: u8) -> Result<()> {
    let locker_state: voter::Locker = program.account(locker)?;
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let (vote, _bump) = Pubkey::find_program_address(
        &[
            b"MeteoraVote".as_ref(),
            proposal.as_ref(),
            program.payer().as_ref(),
        ],
        &govern::id(),
    );
    let mut instructions = vec![];
    if program.rpc().get_account_data(&vote).is_err() {
        instructions.push(Instruction {
            program_id: govern::id(),
            accounts: govern::accounts::NewVote {
                proposal,
                vote,
                payer: program.payer(),
                system_program: solana_program::system_program::ID,
            }
            .to_account_metas(None),
            data: govern::instruction::NewVote {
                voter: program.payer(),
            }
            .data(),
        });
    }
    instructions.push(Instruction {
        program_id: voter::id(),
        accounts: voter::accounts::CastVote {
            locker,
            escrow,
            vote,
            proposal,
            vote_delegate: program.payer(),
            governor: locker_state.governor,
            govern_program: govern::ID,
        }
        .to_account_metas(None),
        data: voter::instruction::CastVote { side }.data(),
    });

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

fn set_vote_delegate(program: &Program, locker: Pubkey, new_delegate: Pubkey) -> Result<()> {
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let builder = program
        .request()
        .accounts(voter::accounts::SetVoteDelegate {
            escrow,
            escrow_owner: program.payer(),
        })
        .args(voter::instruction::SetVoteDelegate { new_delegate });
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

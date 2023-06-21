mod args;

use crate::args::*;
use anyhow::Ok;
use anyhow::Result;
use solana_program::system_program;
use utils_cli::*;

use anchor_client::anchor_lang::InstructionData;
use anchor_client::anchor_lang::ToAccountMetas;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signer::keypair::*;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::{Client, Program};
use clap::*;
use solana_program::instruction::Instruction;
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
        CliCommand::CreateGaugeFactory {
            epoch_duration_seconds,
            first_epoch_starts_at,
        } => {
            create_gauge_factory(
                &program,
                epoch_duration_seconds,
                first_epoch_starts_at,
                base,
            )?;
        }
        CliCommand::CreateGauge { token_mint } => {
            create_gauge(&program, token_mint, base.pubkey())?;
        }
        CliCommand::CreateGaugeVoter {} => {
            create_gauge_voter(&program, base.pubkey())?;
        }
        CliCommand::CreateGaugeVote { token_mint } => {
            create_gauge_vote(&program, token_mint, base.pubkey())?;
        }
        CliCommand::CreateEpochGauge {
            token_mint,
            voting_epoch,
        } => {
            create_epoch_gauge(&program, voting_epoch, token_mint, base.pubkey())?;
        }
        CliCommand::PrepareEpochGaugeVoter {} => {
            prepare_epoch_gauge_voter(&program, base.pubkey())?;
        }
        CliCommand::ResetEpochGaugeVoter {} => {
            reset_epoch_gauge_voter(&program, base.pubkey())?;
        }
        CliCommand::GaugeSetVote { token_mint, weight } => {
            gauge_set_vote(&program, weight, token_mint, base.pubkey())?;
        }
        CliCommand::GaugeCommitVote { token_mint } => {
            gauge_commit_vote(&program, token_mint, base.pubkey())?;
        }
        CliCommand::GaugeRevertVote { token_mint } => {
            gauge_revert_vote(&program, token_mint, base.pubkey())?;
        }
        CliCommand::GaugeEnable { token_mint } => {
            gauge_enable(&program, token_mint, base.pubkey())?;
        }
        CliCommand::GaugeDisable { token_mint } => {
            gauge_disable(&program, token_mint, base.pubkey())?;
        }
        CliCommand::TriggerNextEpoch {} => {
            trigger_next_epoch(&program, base.pubkey())?;
        }
        CliCommand::SyncGauge { token_mint } => {
            sync_gauge(&program, token_mint, base.pubkey())?;
        }
        CliCommand::SyncDisabledGauge { token_mint } => {
            sync_disable_gauge(&program, token_mint, base.pubkey())?;
        }
        CliCommand::CloseEpochGaugeVote {
            token_mint,
            voting_epoch,
        } => {
            close_epoch_gauge(&program, voting_epoch, token_mint, base.pubkey())?;
        }
    }

    Ok(())
}

fn create_gauge_factory(
    program: &Program,
    epoch_duration_seconds: u32,
    first_epoch_starts_at: u64,
    base_kp: Keypair,
) -> Result<()> {
    let base = base_kp.pubkey();

    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());

    let builder = program
        .request()
        .accounts(gauge::accounts::CreateGaugeFactory {
            base,
            gauge_factory,
            rewarder,
            locker,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(gauge::instruction::CreateGaugeFactory {
            foreman: program.payer(),
            epoch_duration_seconds,
            first_epoch_starts_at,
        })
        .signer(&base_kp);
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn create_gauge(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );

    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let gauge_state: gauge::Gauge = program.account(gauge)?;

    let builder = program
        .request()
        .accounts(gauge::accounts::CreateGauge {
            quarry,
            gauge,
            amm_pool: gauge_state.amm_pool,
            gauge_factory,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(gauge::instruction::CreateGauge {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn create_gauge_voter(program: &Program, base: Pubkey) -> Result<()> {
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
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
                locker: locker,
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

    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    instructions.push(Instruction {
        accounts: gauge::accounts::CreateGaugeVoter {
            gauge_voter,
            gauge_factory,
            escrow,
            payer: program.payer(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: gauge::instruction::CreateGaugeVoter {}.data(),
        program_id: gauge::id(),
    });

    let builder = program.request();
    let builder = instructions
        .into_iter()
        .fold(builder, |bld, ix| bld.instruction(ix));

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn create_gauge_vote(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );
    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );
    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    let (gauge_vote, _bump) = Pubkey::find_program_address(
        &[b"GaugeVote".as_ref(), gauge_voter.as_ref(), gauge.as_ref()],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::CreateGaugeVote {
            gauge_vote,
            gauge_voter,
            gauge,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(gauge::instruction::CreateGaugeVote {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn create_epoch_gauge(
    program: &Program,
    voting_epoch: u32,
    token_mint: Pubkey,
    base: Pubkey,
) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let (epoch_gauge, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGauge".as_ref(),
            gauge.as_ref(),
            voting_epoch.to_le_bytes().as_ref(),
        ],
        &gauge::id(),
    );

    let gauge_state: gauge::Gauge = program.account(gauge)?;

    let builder = program
        .request()
        .accounts(gauge::accounts::CreateEpochGauge {
            gauge_factory,
            gauge,
            epoch_gauge,
            amm_pool: gauge_state.amm_pool,
            token_a_fee: gauge_state.token_a_fee_key,
            token_b_fee: gauge_state.token_b_fee_key,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(gauge::instruction::CreateEpochGauge {});
    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn prepare_epoch_gauge_voter(program: &Program, base: Pubkey) -> Result<()> {
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );
    let mut instructions = vec![];

    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory)?;

    let (epoch_gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVoter".as_ref(),
            gauge_voter.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    instructions.push(Instruction {
        accounts: gauge::accounts::PrepareEpochGaugeVoter {
            locker,
            gauge_voter,
            gauge_factory,
            epoch_gauge_voter,
            escrow,
            payer: program.payer(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: gauge::instruction::PrepareEpochGaugeVoter {}.data(),
        program_id: gauge::id(),
    });

    let builder = program.request();
    let builder = instructions
        .into_iter()
        .fold(builder, |bld, ix| bld.instruction(ix));

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn reset_epoch_gauge_voter(program: &Program, base: Pubkey) -> Result<()> {
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );
    let mut instructions = vec![];

    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory)?;

    let (epoch_gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVoter".as_ref(),
            gauge_voter.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    instructions.push(Instruction {
        accounts: gauge::accounts::ResetEpochGaugeVoter {
            locker,
            gauge_voter,
            gauge_factory,
            epoch_gauge_voter,
            escrow,
        }
        .to_account_metas(None),
        data: gauge::instruction::ResetEpochGaugeVoter {}.data(),
        program_id: gauge::id(),
    });

    let builder = program.request();
    let builder = instructions
        .into_iter()
        .fold(builder, |bld, ix| bld.instruction(ix));

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn gauge_set_vote(program: &Program, weight: u32, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let mut instructions = vec![];
    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );
    let gauge_voter_info = program.rpc().get_account(&gauge_voter);
    if gauge_voter_info.is_err() {
        instructions.push(Instruction {
            accounts: gauge::accounts::CreateGaugeVoter {
                gauge_voter,
                gauge_factory,
                escrow,
                payer: program.payer(),
                system_program: system_program::id(),
            }
            .to_account_metas(None),
            data: gauge::instruction::CreateGaugeVoter {}.data(),
            program_id: gauge::id(),
        });
    }

    let (gauge_vote, _bump) = Pubkey::find_program_address(
        &[b"GaugeVote".as_ref(), gauge_voter.as_ref(), gauge.as_ref()],
        &gauge::id(),
    );
    let gauge_vote_info = program.rpc().get_account(&gauge_vote);
    if gauge_vote_info.is_err() {
        instructions.push(Instruction {
            accounts: gauge::accounts::CreateGaugeVote {
                gauge_vote,
                gauge_voter,
                gauge,
                payer: program.payer(),
                system_program: system_program::id(),
            }
            .to_account_metas(None),
            data: gauge::instruction::CreateGaugeVote {}.data(),
            program_id: gauge::id(),
        });
    }
    instructions.push(Instruction {
        accounts: gauge::accounts::GaugeSetVote {
            gauge_factory,
            escrow,
            gauge,
            gauge_voter,
            gauge_vote,
            vote_delegate: program.payer(),
        }
        .to_account_metas(None),
        data: gauge::instruction::GaugeSetVote { weight }.data(),
        program_id: gauge::id(),
    });

    let builder = program.request();
    let builder = instructions
        .into_iter()
        .fold(builder, |bld, ix| bld.instruction(ix));

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn gauge_commit_vote(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let gauge_state: gauge::Gauge = program.account(gauge)?;

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    let (gauge_vote, _bump) = Pubkey::find_program_address(
        &[b"GaugeVote".as_ref(), gauge_voter.as_ref(), gauge.as_ref()],
        &gauge::id(),
    );

    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory)?;

    let (epoch_gauge, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGauge".as_ref(),
            gauge.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    let mut instructions = vec![];
    let epoch_gauge_info = program.rpc().get_account(&epoch_gauge);
    if epoch_gauge_info.is_err() {
        instructions.push(Instruction {
            accounts: gauge::accounts::CreateEpochGauge {
                gauge_factory,
                gauge,
                amm_pool: gauge_state.amm_pool,
                token_a_fee: gauge_state.token_a_fee_key,
                token_b_fee: gauge_state.token_b_fee_key,
                epoch_gauge,
                payer: program.payer(),
                system_program: system_program::id(),
            }
            .to_account_metas(None),
            data: gauge::instruction::CreateEpochGauge {}.data(),
            program_id: gauge::id(),
        });
    }

    let (epoch_gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVoter".as_ref(),
            gauge_voter.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    let (epoch_gauge_vote, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVote".as_ref(),
            gauge_vote.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    instructions.push(Instruction {
        accounts: gauge::accounts::GaugeCommitVote {
            gauge_factory,
            gauge,
            gauge_voter,
            gauge_vote,
            epoch_gauge_voter,
            epoch_gauge,
            payer: program.payer(),
            epoch_gauge_vote,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: gauge::instruction::GaugeCommitVote {}.data(),
        program_id: gauge::id(),
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

fn gauge_revert_vote(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    let (gauge_vote, _bump) = Pubkey::find_program_address(
        &[b"GaugeVote".as_ref(), gauge_voter.as_ref(), gauge.as_ref()],
        &gauge::id(),
    );

    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory)?;

    let (epoch_gauge, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGauge".as_ref(),
            gauge.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    let (epoch_gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVoter".as_ref(),
            gauge_voter.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    let (epoch_gauge_vote, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVote".as_ref(),
            gauge_vote.as_ref(),
            gauge_factory_state
                .current_voting_epoch
                .to_le_bytes()
                .as_ref(),
        ],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::GaugeRevertVote {
            escrow,
            gauge_factory,
            gauge,
            gauge_voter,
            gauge_vote,
            epoch_gauge_voter,
            epoch_gauge,
            payer: program.payer(),
            vote_delegate: program.payer(),
            epoch_gauge_vote,
        })
        .args(gauge::instruction::GaugeRevertVote {});

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn gauge_enable(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::GaugeEnable {
            gauge_factory,
            gauge,
            foreman: program.payer(),
        })
        .args(gauge::instruction::GaugeEnable {});

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn gauge_disable(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::GaugeDisable {
            gauge_factory,
            gauge,
            foreman: program.payer(),
        })
        .args(gauge::instruction::GaugeDisable {});

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn trigger_next_epoch(program: &Program, base: Pubkey) -> Result<()> {
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let builder = program
        .request()
        .accounts(gauge::accounts::TriggerNextEpoch { gauge_factory })
        .args(gauge::instruction::TriggerNextEpoch {});

    // let result = simulate_transaction(&builder, program, &vec![&default_keypair()]).unwrap();
    // println!("{:?}", result);
    // return Ok(());

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn sync_gauge(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory)?;

    let (epoch_gauge, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGauge".as_ref(),
            gauge.as_ref(),
            gauge_factory_state.rewards_epoch().to_le_bytes().as_ref(),
        ],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::SyncGauge {
            gauge_factory,
            gauge,
            epoch_gauge,
            quarry,
            rewarder,
            quarry_program: quarry::id(),
        })
        .args(gauge::instruction::SyncGauge {});

    // let result = simulate_transaction(&builder, program, &vec![&default_keypair()]).unwrap();
    // println!("{:?}", result);
    // return Ok(());

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn sync_disable_gauge(program: &Program, token_mint: Pubkey, base: Pubkey) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::SyncDisabledGauge {
            gauge_factory,
            gauge,
            quarry,
            rewarder,
            quarry_program: quarry::id(),
        })
        .args(gauge::instruction::SyncDisabledGauge {});

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

fn close_epoch_gauge(
    program: &Program,
    voting_epoch: u32,
    token_mint: Pubkey,
    base: Pubkey,
) -> Result<()> {
    let (rewarder, _bump) =
        Pubkey::find_program_address(&[b"Rewarder".as_ref(), base.as_ref()], &quarry::id());
    let (quarry, _bump) = Pubkey::find_program_address(
        &[b"Quarry".as_ref(), rewarder.as_ref(), token_mint.as_ref()],
        &quarry::id(),
    );
    let (gauge_factory, _bump) =
        Pubkey::find_program_address(&[b"GaugeFactory".as_ref(), base.as_ref()], &gauge::id());

    let (gauge, _bump) = Pubkey::find_program_address(
        &[b"Gauge".as_ref(), gauge_factory.as_ref(), quarry.as_ref()],
        &gauge::id(),
    );

    let (locker, _bump) =
        Pubkey::find_program_address(&[b"Locker".as_ref(), base.as_ref()], &voter::id());
    let (escrow, _bump) = Pubkey::find_program_address(
        &[
            b"Escrow".as_ref(),
            locker.as_ref(),
            program.payer().as_ref(),
        ],
        &voter::id(),
    );

    let (gauge_voter, _bump) = Pubkey::find_program_address(
        &[
            b"GaugeVoter".as_ref(),
            gauge_factory.as_ref(),
            escrow.as_ref(),
        ],
        &gauge::id(),
    );

    let (gauge_vote, _bump) = Pubkey::find_program_address(
        &[b"GaugeVote".as_ref(), gauge_voter.as_ref(), gauge.as_ref()],
        &gauge::id(),
    );

    let (epoch_gauge_vote, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGaugeVote".as_ref(),
            gauge_vote.as_ref(),
            voting_epoch.to_le_bytes().as_ref(),
        ],
        &gauge::id(),
    );

    let builder = program
        .request()
        .accounts(gauge::accounts::CloseEpochGaugeVote {
            gauge_factory,
            gauge,
            gauge_voter,
            gauge_vote,
            epoch_gauge_vote,
            escrow,
            recipient: program.payer(),
            vote_delegate: program.payer(),
        })
        .args(gauge::instruction::CloseEpochGaugeVote { voting_epoch });

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

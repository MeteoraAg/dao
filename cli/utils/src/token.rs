use anchor_client::solana_sdk::program_pack::Pack;
use anchor_client::solana_sdk::signature::Signer;
use anchor_client::solana_sdk::signer::keypair::*;
use anchor_client::solana_sdk::system_instruction;
use anchor_lang::prelude::Pubkey;
use anchor_lang::prelude::Rent;
use anyhow::Result;

pub fn get_or_create_ata(
    program_client: &anchor_client::Program,
    token_mint: Pubkey,
    user: Pubkey,
) -> Result<Pubkey> {
    let user_token_account =
        spl_associated_token_account::get_associated_token_address(&user, &token_mint);
    let rpc_client = program_client.rpc();
    if rpc_client.get_account_data(&user_token_account).is_err() {
        println!("Create ATA for TOKEN {} \n", &token_mint);

        let builder = program_client.request().instruction(
            spl_associated_token_account::create_associated_token_account(
                &program_client.payer(),
                &user,
                &token_mint,
            ),
        );

        let signature = builder.send()?;
        println!("{}", signature);
    }
    Ok(user_token_account)
}

pub fn create_token_mint(program: &anchor_client::Program, token_mint_kp: &Keypair) -> Result<()> {
    let instructions = vec![
        system_instruction::create_account(
            &program.payer(),
            &token_mint_kp.pubkey(),
            Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &token_mint_kp.pubkey(),
            &program.payer(),
            None, // TODO: Check if the vault should be allowed to freeze, or if it is general pratice to allow swap program to
            9,
        )
        .unwrap(),
    ];
    println!("create token mint {}", token_mint_kp.pubkey());
    let builder = program.request().signer(token_mint_kp);

    let builder = instructions
        .into_iter()
        .fold(builder, |bld, ix| bld.instruction(ix));

    let signature = builder.send()?;
    println!("Signature {:?}", signature);
    Ok(())
}

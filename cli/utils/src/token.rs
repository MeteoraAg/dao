use anchor_lang::prelude::Pubkey;
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

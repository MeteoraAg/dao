
import { Program, web3 } from "@project-serum/anchor";

import { TOKEN_PROGRAM_ID, createMint, setAuthority, AuthorityType } from "@solana/spl-token";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";

import * as anchor from "@project-serum/anchor";
import { Minter } from "../../target/types/minter";

const programMinter = anchor.workspace.Minter as Program<Minter>;
const provider = anchor.AnchorProvider.env();
const BN = anchor.BN;

export async function setupTokenMintAndMinter(baseKP: Keypair, adminKP: Keypair, decimals: number, hardCap: number): Promise<{
    rewardsMint: PublicKey,
    mintWrapper: PublicKey,
}> {
    // create mint
    let rewardsMint = await createMint(
        provider.connection,
        adminKP,
        adminKP.publicKey,
        null,
        decimals
    );

    const [mintWrapper, sBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("MintWrapper"), baseKP.publicKey.toBuffer()],
            programMinter.programId
        );

    await setAuthority(
        provider.connection,
        adminKP,
        rewardsMint,
        adminKP.publicKey,
        AuthorityType.MintTokens,
        mintWrapper,
    );
    await programMinter.methods
        .newWrapper(new BN(hardCap))
        .accounts({
            base: baseKP.publicKey,
            mintWrapper,
            tokenMint: rewardsMint,
            tokenProgram: TOKEN_PROGRAM_ID,
            admin: adminKP.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
        }).signers([baseKP])
        .rpc();

    return {
        rewardsMint,
        mintWrapper,
    }
}
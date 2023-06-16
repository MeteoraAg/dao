import { Program, web3 } from "@project-serum/anchor";

import {
    createAndFundWallet,
} from "../utils";

import { TOKEN_PROGRAM_ID, createMint, mintTo } from "@solana/spl-token";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";

import * as anchor from "@project-serum/anchor";
import { MocAmm } from "../../target/types/moc_amm";
import { BN } from "bn.js";

const programMocAmm = anchor.workspace.MocAmm as Program<MocAmm>;
const provider = anchor.AnchorProvider.env();
export const DEFAULT_DECIMALS = 9;

// admin can mint fee
export async function createMocAmm(fee: number, lpMint: web3.PublicKey, adminKP: Keypair): Promise<PublicKey> {
    let baseKP = new anchor.web3.Keypair();
    var [mocAmm, mBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("moc_amm"), baseKP.publicKey.toBuffer()],
            programMocAmm.programId
        );
    // let adminKP = new anchor.web3.Keypair();
    // await createAndFundWallet(provider.connection, adminKP);
    let tokenMintA = await createMint(
        provider.connection,
        adminKP,
        adminKP.publicKey,
        null,
        DEFAULT_DECIMALS
    );

    var [tokenAFee, mBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("token_a_fee"), mocAmm.toBuffer()],
            programMocAmm.programId
        );
    let tokenMintB = await createMint(
        provider.connection,
        adminKP,
        adminKP.publicKey,
        null,
        DEFAULT_DECIMALS
    );
    var [tokenBFee, mBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("token_b_fee"), mocAmm.toBuffer()],
            programMocAmm.programId
        );

    await programMocAmm.methods.newMocAmm(new BN(fee), lpMint).accounts({
        base: baseKP.publicKey,
        mocAmm,
        tokenMintA,
        tokenAFee,
        tokenMintB,
        tokenBFee,
        tokenProgram: TOKEN_PROGRAM_ID,
        payer: provider.wallet.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
    }).signers([baseKP]).rpc();

    return mocAmm;
}

export async function simulateSwapAtoB(ammPool: PublicKey, amount: number, adminKP: Keypair) {
    // just need to mint more for tokenAFee and tokenBFee
    let ammState = await programMocAmm.account.mocAmm.fetch(ammPool);
    let payerKP = new anchor.web3.Keypair();
    await createAndFundWallet(provider.connection, payerKP);

    let fee = amount * ammState.fee.toNumber() / 10000; // 1% as fee
    console.log("swap fee ", fee);

    await mintTo(provider.connection, adminKP, ammState.tokenAMint, ammState.tokenAFee, adminKP.publicKey, fee);
}

export async function simulateSwapBtoA(ammPool: PublicKey, amount: number, adminKP: Keypair) {
    // just need to mint more for tokenAFee and tokenBFee
    let ammState = await programMocAmm.account.mocAmm.fetch(ammPool);
    let payerKP = new anchor.web3.Keypair();
    await createAndFundWallet(provider.connection, payerKP);

    let fee = amount * ammState.fee.toNumber() / 10000; // 1% as fee
    console.log("swap fee ", fee);

    await mintTo(provider.connection, adminKP, ammState.tokenBMint, ammState.tokenBFee, adminKP.publicKey, fee);
}
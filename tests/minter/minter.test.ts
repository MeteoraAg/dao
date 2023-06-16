import * as anchor from "@project-serum/anchor";
import { Program, web3, Wallet, } from "@project-serum/anchor";
import type { PublicKey } from "@solana/web3.js";
import { expect } from "chai";
import { Minter } from "../../target/types/minter";
import { TOKEN_PROGRAM_ID, createMint, setAuthority, AuthorityType, getMint } from "@solana/spl-token";

import {
  Keypair,
  SystemProgram,
} from "@solana/web3.js";

import {
  createAndFundWallet,
  getOrCreateATA,
  setupTokenMintAndMinter,
} from "../utils";

export const DEFAULT_DECIMALS = 9;
export const DEFAULT_HARD_CAP = 1_000_000_000_000_000;

const BN = anchor.BN;
type BN = anchor.BN;
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.Minter as Program<Minter>;

describe("MintWrapper", () => {
  let adminKP: web3.Keypair;
  let wallet: Wallet;
  let rewardsMint: PublicKey;
  let mintWrapperKey: PublicKey;
  let minterBase: web3.Keypair;

  beforeEach("Initialize mint", async () => {
    minterBase = new anchor.web3.Keypair();
    const result = await createAndFundWallet(provider.connection);
    adminKP = result.keypair;
    wallet = result.wallet;

    let minterResult = await setupTokenMintAndMinter(minterBase, adminKP, DEFAULT_DECIMALS, DEFAULT_HARD_CAP);
    rewardsMint = minterResult.rewardsMint;
    mintWrapperKey = minterResult.mintWrapper;
  });

  it("Check MintWrapper", async () => {
    const mintInfo = await getMint(provider.connection, rewardsMint);
    expect(mintInfo.mintAuthority).to.deep.equal(mintWrapperKey);

    const mintWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);
    expect(mintWrapperState.hardCap.toNumber()).to.equal(DEFAULT_HARD_CAP);
    expect(mintWrapperState.admin).to.deep.equal(adminKP.publicKey);
    expect(mintWrapperState.tokenMint).to.deep.equal(rewardsMint);
  });

  it("Transfer admin authority and accept admin authority", async () => {
    const newAuthority = web3.Keypair.generate();

    await program.methods
      .transferAdmin()
      .accounts({
        mintWrapper: mintWrapperKey,
        admin: adminKP.publicKey,
        nextAdmin: newAuthority.publicKey,
      }).signers([adminKP])
      .rpc();


    let mintWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);

    expect(mintWrapperState.admin).to.deep.equal(adminKP.publicKey);
    expect(mintWrapperState.pendingAdmin).to.deep.equal(
      newAuthority.publicKey
    );

    await program.methods
      .acceptAdmin()
      .accounts({
        mintWrapper: mintWrapperKey,
        pendingAdmin: newAuthority.publicKey,
      }).signers([newAuthority])
      .rpc();

    mintWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);

    expect(mintWrapperState.admin).to.deep.equal(newAuthority.publicKey);
    expect(mintWrapperState.pendingAdmin).to.deep.equal(
      web3.PublicKey.default
    );

    // Transfer back
    await program.methods
      .transferAdmin()
      .accounts({
        mintWrapper: mintWrapperKey,
        admin: newAuthority.publicKey,
        nextAdmin: adminKP.publicKey,
      }).signers([newAuthority])
      .rpc();

    await program.methods
      .acceptAdmin()
      .accounts({
        mintWrapper: mintWrapperKey,
        pendingAdmin: adminKP.publicKey,
      }).signers([adminKP])
      .rpc();


    mintWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);

    expect(mintWrapperState.admin).to.deep.equal(adminKP.publicKey);
    expect(mintWrapperState.pendingAdmin).to.deep.equal(
      web3.PublicKey.default
    );
  });

  it("Adds a Minter", async () => {
    const minterAuthority = Keypair.generate().publicKey;
    const [minter, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MintWrapperMinter"), mintWrapperKey.toBuffer(), minterAuthority.toBuffer()],
        program.programId
      );

    await program.methods
      .newMinter()
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minterAuthority,
        minter,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).signers([adminKP])
      .rpc();

    let minterWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);
    expect(minterWrapperState.numMinters.toNumber()).to.deep.equal(1);

    // set allowance
    const allowance = 1_000_000;
    await program.methods
      .minterUpdate(new BN(allowance))
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minter,
      }).signers([adminKP])
      .rpc();

    let minterState =
      await program.account.minter.fetch(minter);
    expect(minterState.allowance.toNumber()).to.deep.equal(allowance);

    minterWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);
    expect(minterWrapperState.totalAllowance.toNumber()).to.deep.equal(allowance);
  });

  it("Removes a Minter", async () => {
    const minterAuthority = Keypair.generate().publicKey;
    const [minter, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MintWrapperMinter"), mintWrapperKey.toBuffer(), minterAuthority.toBuffer()],
        program.programId
      );

    await program.methods
      .newMinter()
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minterAuthority,
        minter,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).signers([adminKP])
      .rpc();

    // set allowance
    const allowance = 1_000_000;
    await program.methods
      .minterUpdate(new BN(allowance))
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minter,
      }).signers([adminKP])
      .rpc();

    // remove minter
    await program.methods
      .minterUpdate(new BN(0))
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minter,
      }).signers([adminKP])
      .rpc();
    let minterState =
      await program.account.minter.fetch(minter);
    expect(minterState.allowance.toNumber()).to.deep.equal(0);


  });

  it("Cannot mint past allowance", async () => {
    let minterWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);

    const minterAuthorityKP = Keypair.generate();
    const minterAuthority = minterAuthorityKP.publicKey;
    const [minter, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MintWrapperMinter"), mintWrapperKey.toBuffer(), minterAuthority.toBuffer()],
        program.programId
      );

    await program.methods
      .newMinter()
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minterAuthority,
        minter,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).signers([adminKP])
      .rpc();

    // set allowance
    const allowance = 1_000_000;
    await program.methods
      .minterUpdate(new BN(allowance))
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minter,
      }).signers([adminKP])
      .rpc();


    let destination = await getOrCreateATA(
      minterWrapperState.tokenMint,
      provider.wallet.publicKey,
      adminKP,
      provider.connection
    );

    // mint
    const amount = 1_000;
    await program.methods.performMint(new BN(amount)).accounts({
      mintWrapper: mintWrapperKey,
      minterAuthority,
      tokenMint: minterWrapperState.tokenMint,
      minter,
      destination,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([minterAuthorityKP]).rpc();


    minterWrapperState =
      await program.account.mintWrapper.fetch(mintWrapperKey);
    expect(minterWrapperState.totalAllowance.toNumber()).to.deep.equal(allowance - amount);
    expect(minterWrapperState.totalMinted.toNumber()).to.deep.equal(amount);


    let minterState =
      await program.account.minter.fetch(minter);
    expect(minterState.allowance.toNumber()).to.deep.equal(allowance - amount);
    expect(minterState.totalMinted.toNumber()).to.deep.equal(amount);

    try {
      await program.methods.performMint(new BN(allowance)).accounts({
        mintWrapper: mintWrapperKey,
        minterAuthority,
        tokenMint: minterWrapperState.tokenMint,
        minter,
        destination,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([minterAuthorityKP]).rpc();
      expect(1).to.deep.equal(0);
    } catch (e) {
      console.log("Cannot mint more than allowance")
    }
  });
});

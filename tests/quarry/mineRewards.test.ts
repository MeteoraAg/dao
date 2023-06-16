import * as anchor from "@project-serum/anchor";
import { Program, web3, Wallet, BN } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint, setAuthority, AuthorityType, getMint, mintTo } from "@solana/spl-token";

import {
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { expect } from "chai";
import {
  createAndFundWallet,
  createMocAmm,
  getOrCreateATA,
  sleep,
  setupTokenMintAndMinter
} from "../utils";

import { Quarry } from "../../target/types/quarry";
import { Minter } from "../../target/types/minter";

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.Quarry as Program<Quarry>;
const programMinter = anchor.workspace.Minter as Program<Minter>;

export const DEFAULT_DECIMALS = 9;
export const DEFAULT_HARD_CAP = 1_000_000_000_000_000;

describe("Mine Rewards", () => {
  const dailyRewardsRate = new BN(1_000 * LAMPORTS_PER_SOL);
  const annualRewardsRate = dailyRewardsRate.mul(new BN(365));

  const stakeAmount = 1_000_000000;

  let adminKP: web3.Keypair;
  let wallet: Wallet;
  let stakeTokenMint: PublicKey;
  let ammPool: PublicKey;
  let stakedMintAuthority: anchor.web3.Keypair;


  before(async () => {
    const result = await createAndFundWallet(provider.connection);
    adminKP = result.keypair;
    wallet = result.wallet;
    // create mint
    stakedMintAuthority = web3.Keypair.generate();
    stakeTokenMint = await createMint(
      provider.connection,
      adminKP,
      stakedMintAuthority.publicKey,
      null,
      DEFAULT_DECIMALS
    );
    ammPool = await createMocAmm(stakeTokenMint);
  });

  let rewardsMint: PublicKey;
  let mintWrapperKey: PublicKey;
  let minterBase: web3.Keypair;

  beforeEach("Initialize minter", async () => {
    minterBase = new anchor.web3.Keypair();

    let minterResult = await setupTokenMintAndMinter(minterBase, adminKP, DEFAULT_DECIMALS, DEFAULT_HARD_CAP);
    rewardsMint = minterResult.rewardsMint;
    mintWrapperKey = minterResult.mintWrapper;
  });

  let rewarderKey: PublicKey;
  beforeEach("Set up rewarder", async () => {
    let rewarderBaseKP = new anchor.web3.Keypair();
    let rewarderBase = rewarderBaseKP.publicKey;
    const [rewarder, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("Rewarder"), rewarderBase.toBuffer()],
        program.programId
      );
    await program.methods.newRewarder().accounts({
      base: rewarderBase,
      rewarder,
      admin: adminKP.publicKey,
      payer: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      mintWrapper: mintWrapperKey,
      rewardsTokenMint: rewardsMint,
    }).signers([rewarderBaseKP, adminKP]).rpc();
    rewarderKey = rewarder;

    const rewarderState = await program.account.rewarder.fetch(rewarderKey);
    expect(rewarderState.rewardsTokenMint).to.deep.equal(rewardsMint);

    await program.methods.setAnnualRewards(annualRewardsRate).accounts({
      auth: {
        admin: adminKP.publicKey,
        rewarder: rewarderKey,
      }
    }).signers([adminKP]).rpc();

    // whitelist rewarder
    const [minter, mBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MintWrapperMinter"), mintWrapperKey.toBuffer(), rewarderKey.toBuffer()],
        programMinter.programId
      );
    await programMinter.methods
      .newMinter()
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minterAuthority: rewarderKey,
        minter,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).signers([adminKP])
      .rpc();
    // set allowance
    const allowance = 100_000_000_000_000;
    await programMinter.methods
      .minterUpdate(new BN(allowance))
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: adminKP.publicKey,
        },
        minter,
      }).signers([adminKP])
      .rpc();
  });

  let quarryKey: PublicKey;
  let minerKey: PublicKey;

  beforeEach("Set up quarry and miner", async () => {
    const [quarry, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("Quarry"), rewarderKey.toBuffer(), ammPool.toBuffer()],
        program.programId
      );
    quarryKey = quarry;
    await program.methods.createQuarry().accounts({
      quarry,
      auth: {
        admin: adminKP.publicKey,
        rewarder: rewarderKey,
      },
      ammPool,
      payer: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
    }).signers([adminKP]).rpc();
    // mint test tokens
    let userStakeTokenAccount = await getOrCreateATA(
      stakeTokenMint,
      provider.wallet.publicKey,
      adminKP,
      provider.connection
    );

    await mintTo(
      provider.connection,
      adminKP,
      stakeTokenMint,
      userStakeTokenAccount,
      stakedMintAuthority,
      stakeAmount
    );
    await program.methods.setRewardsShare(new BN(100)).accounts({
      quarry,
      authority: adminKP.publicKey,
      rewarder: rewarderKey,
    }).signers([adminKP]).rpc();

    const [miner, mbump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("Miner"), quarry.toBuffer(), provider.wallet.publicKey.toBuffer()],
        program.programId
      );
    minerKey = miner;

    const [minerVault, bump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MinerVault"), minerKey.toBuffer()],
        program.programId
      );

    await program.methods.createMiner().accounts({
      authority: provider.wallet.publicKey,
      miner,
      quarry,
      rewarder: rewarderKey,
      payer: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      tokenMint: stakeTokenMint,
      minerVault,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).rpc();
  });


  it("Does not lose rewards when setting quarry share", async () => {
    const [minerVault, bump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MinerVault"), minerKey.toBuffer()],
        program.programId
      );
    let userStakeTokenAccount = await getOrCreateATA(
      stakeTokenMint,
      provider.wallet.publicKey,
      adminKP,
      provider.connection
    );
    await program.methods.stakeTokens(new BN(stakeAmount)).accounts({
      authority: provider.wallet.publicKey,
      miner: minerKey,
      quarry: quarryKey,
      minerVault,
      tokenAccount: userStakeTokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
      rewarder: rewarderKey,
    }).rpc();

    // wait some time so we can earn some tokens
    await sleep(3_000);

    await program.methods.setRewardsShare(new BN(0)).accounts({
      quarry: quarryKey,
      authority: adminKP.publicKey,
      rewarder: rewarderKey,
    }).signers([adminKP]).rpc();

    // update reward share
    await program.methods.updateQuarryRewards().accounts({
      quarry: quarryKey,
      rewarder: rewarderKey,
    }).rpc();



    const [minter, mBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MintWrapperMinter"), mintWrapperKey.toBuffer(), rewarderKey.toBuffer()],
        programMinter.programId
      );
    let rewardsTokenAccount = await getOrCreateATA(
      rewardsMint,
      provider.wallet.publicKey,
      adminKP,
      provider.connection
    );

    await program.methods.claimRewards().accounts({
      mintWrapper: mintWrapperKey,
      mintWrapperProgram: programMinter.programId,
      minter,
      rewardsTokenMint: rewardsMint,
      rewardsTokenAccount,
      claim: {
        authority: provider.wallet.publicKey,
        miner: minerKey,
        quarry: quarryKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        rewarder: rewarderKey,
      }
    }).rpc();

    let minerBalance = await provider.connection
      .getTokenAccountBalance(rewardsTokenAccount);

    expect(Number(minerBalance.value.amount)).not.to.equal(0);
  });
});

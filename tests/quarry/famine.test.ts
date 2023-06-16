import * as anchor from "@project-serum/anchor";
import { Program, web3, Wallet, BN } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint, setAuthority, AuthorityType, mintTo } from "@solana/spl-token";

import {
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { expect } from "chai";
import {
  createAndFundWallet,
  getOrCreateATA,
  sleep,
  createMocAmm,
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

describe("Famine", () => {
  const stakeAmount = 1_000_000_000;
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
    ammPool = await createMocAmm(30, stakeTokenMint, adminKP);
  });

  let rewardsMint: PublicKey;
  let mintWrapperKey: PublicKey;

  beforeEach("Initialize minter", async () => {
    let minterBase = new anchor.web3.Keypair();


    let minterResult = await setupTokenMintAndMinter(minterBase, adminKP, DEFAULT_DECIMALS, DEFAULT_HARD_CAP);
    rewardsMint = minterResult.rewardsMint;
    mintWrapperKey = minterResult.mintWrapper;
  });

  // let rewarderWrapper: RewarderWrapper;
  const dailyRewardsRate = new BN(1_000_000 * DEFAULT_DECIMALS);
  const annualRewardsRate = dailyRewardsRate.mul(new BN(365));

  let rewarderKey: PublicKey;
  beforeEach("Set up rewarder", async () => {
    let rewarderBaseKP = new anchor.web3.Keypair();
    let rewarderBase = rewarderBaseKP.publicKey;
    const [rewarder, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("Rewarder"), rewarderBase.toBuffer()],
        program.programId
      );
    rewarderKey = rewarder;
    await program.methods.newRewarder().accounts({
      base: rewarderBase,
      rewarder,
      admin: adminKP.publicKey,
      payer: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      mintWrapper: mintWrapperKey,
      rewardsTokenMint: rewardsMint,
    }).signers([rewarderBaseKP, adminKP]).rpc();


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
        [Buffer.from("MinerVault"), miner.toBuffer()],
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

  it("Stake and claim after famine", async () => {

    const famine = new BN(Date.now() / 1000 - 5); // Rewards stopped 5 seconds ago
    await program.methods.setFamine(famine).accounts({
      quarry: quarryKey,
      auth: {
        admin: adminKP.publicKey,
        rewarder: rewarderKey,
      },
    }).signers([adminKP]).rpc();

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

    // Sleep for 5 seconds
    await sleep(5000);

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
    expect(Number(minerBalance.value.amount)).to.deep.equal(0);
  });

  it("Stake before famine and claim after famine", async () => {
    const rewardsDuration = 5; // 5 seconds
    const famine = new BN(Date.now() / 1_000 + rewardsDuration);
    await program.methods.setFamine(famine).accounts({
      quarry: quarryKey,
      auth: {
        admin: adminKP.publicKey,
        rewarder: rewarderKey,
      },
    }).signers([adminKP]).rpc();

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
    // Sleep for 8 seconds
    await sleep(8_000);

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

    const expectedRewards = dailyRewardsRate
      .div(new BN(86400))
      .mul(new BN(rewardsDuration));

    const prevBalance = minerBalance.value.amount;

    expect(Number(minerBalance.value.amount)).to.closeTo(expectedRewards.toNumber(), 2);

    console.log("Claiming again after 5 seconds ...");
    // Sleep for 5 seconds
    await sleep(5_000);

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

    minerBalance = await provider.connection
      .getTokenAccountBalance(rewardsTokenAccount);
    expect(minerBalance.value.amount).to.be.equal(prevBalance);
  });
});

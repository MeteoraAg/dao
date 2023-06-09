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
  getOrCreateATA,
  sleep,
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

  const rewardsShare = dailyRewardsRate.div(new BN(10));
  const stakeAmount = 1_000_000000;

  let keypair: web3.Keypair;
  let wallet: Wallet;
  let stakeTokenMint: PublicKey;
  let stakedMintAuthority: anchor.web3.Keypair;


  before(async () => {
    const result = await createAndFundWallet(provider.connection);
    keypair = result.keypair;
    wallet = result.wallet;
    // create mint
    stakedMintAuthority = web3.Keypair.generate();
    stakeTokenMint = await createMint(
      provider.connection,
      keypair,
      stakedMintAuthority.publicKey,
      null,
      DEFAULT_DECIMALS
    );
  });

  let rewardsMint: PublicKey;
  let mintWrapperKey: PublicKey;
  let minterBase: web3.Keypair;

  beforeEach("Initialize minter", async () => {
    minterBase = new anchor.web3.Keypair();

    // create mint
    rewardsMint = await createMint(
      provider.connection,
      keypair,
      keypair.publicKey,
      null,
      DEFAULT_DECIMALS
    );

    const [mintWrapper, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("MintWrapper"), minterBase.publicKey.toBuffer()],
        programMinter.programId
      );

    await setAuthority(
      provider.connection,
      keypair,
      rewardsMint,
      keypair.publicKey,
      AuthorityType.MintTokens,
      mintWrapper,
    );

    await programMinter.methods
      .newWrapper(new BN(DEFAULT_HARD_CAP))
      .accounts({
        base: minterBase.publicKey,
        mintWrapper: mintWrapper,
        tokenMint: rewardsMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        admin: provider.wallet.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).signers([minterBase])
      .rpc();

    mintWrapperKey = mintWrapper;
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
      initialAuthority: provider.wallet.publicKey,
      payer: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      mintWrapper: mintWrapperKey,
      rewardsTokenMint: rewardsMint,
    }).signers([rewarderBaseKP]).rpc();
    rewarderKey = rewarder;

    const rewarderState = await program.account.rewarder.fetch(rewarderKey);
    expect(rewarderState.rewardsTokenMint).to.deep.equal(rewardsMint);

    await program.methods.setAnnualRewards(annualRewardsRate).accounts({
      auth: {
        authority: provider.wallet.publicKey,
        rewarder: rewarderKey,
      }
    }).rpc();

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
          admin: provider.wallet.publicKey,
        },
        minterAuthority: rewarderKey,
        minter,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    // set allowance
    const allowance = 100_000_000_000_000;
    await programMinter.methods
      .minterUpdate(new BN(allowance))
      .accounts({
        auth: {
          mintWrapper: mintWrapperKey,
          admin: provider.wallet.publicKey,
        },
        minter,
      })
      .rpc();
  });

  let quarryKey: PublicKey;
  let minerKey: PublicKey;

  beforeEach("Set up quarry and miner", async () => {
    const [quarry, sBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("Quarry"), rewarderKey.toBuffer(), stakeTokenMint.toBuffer()],
        program.programId
      );
    quarryKey = quarry;
    await program.methods.createQuarry().accounts({
      quarry,
      auth: {
        authority: provider.wallet.publicKey,
        rewarder: rewarderKey,
      },
      tokenMint: stakeTokenMint,
      payer: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
    }).rpc();
    // mint test tokens
    let userStakeTokenAccount = await getOrCreateATA(
      stakeTokenMint,
      provider.wallet.publicKey,
      keypair,
      provider.connection
    );

    await mintTo(
      provider.connection,
      keypair,
      stakeTokenMint,
      userStakeTokenAccount,
      stakedMintAuthority,
      stakeAmount
    );
    await program.methods.setRewardsShare(new BN(100)).accounts({
      quarry,
      authority: provider.wallet.publicKey,
      rewarder: rewarderKey,
    }).rpc();

    const [miner, mbump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from("Miner"), quarry.toBuffer(), provider.wallet.publicKey.toBuffer()],
        program.programId
      );
    minerKey = miner;

    let minerVault = await getOrCreateATA(
      stakeTokenMint,
      miner,
      keypair,
      provider.connection
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
    let minerVault = await getOrCreateATA(
      stakeTokenMint,
      minerKey,
      keypair,
      provider.connection
    );
    let userStakeTokenAccount = await getOrCreateATA(
      stakeTokenMint,
      provider.wallet.publicKey,
      keypair,
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
      authority: provider.wallet.publicKey,
      rewarder: rewarderKey,
    }).rpc();

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
      keypair,
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

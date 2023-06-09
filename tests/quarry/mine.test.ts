import * as anchor from "@project-serum/anchor";
import { Program, web3, Wallet, } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint, setAuthority, AuthorityType, getMint, mintTo } from "@solana/spl-token";

import {
  PublicKey,
  SystemProgram,
  Keypair,
} from "@solana/web3.js";
import { expect } from "chai";
import invariant from "tiny-invariant";
import {
  createAndFundWallet,
  getOrCreateATA,
} from "../utils";

import { Quarry } from "../../target/types/quarry";
import { Minter } from "../../target/types/minter";



const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.Quarry as Program<Quarry>;
const programMinter = anchor.workspace.Minter as Program<Minter>;

export const DEFAULT_DECIMALS = 9;
export const DEFAULT_HARD_CAP = 1_000_000_000_000_000;

describe("Mine", () => {
  const { web3, BN } = anchor;

  const DAILY_REWARDS_RATE = new BN(1_000 * web3.LAMPORTS_PER_SOL);
  const ANNUAL_REWARDS_RATE = DAILY_REWARDS_RATE.mul(new BN(365));

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

    // const rewardsMintKP = Keypair.generate();
    // rewardsMint = rewardsMintKP.publicKey;

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

  describe("Rewarder", () => {
    let rewarderKey: PublicKey;

    beforeEach("rewarder", async () => {
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
    });

    it("Is initialized!", async () => {
      const rewarder = await program.account.rewarder.fetch(rewarderKey);

      expect(rewarder.authority).to.deep.equal(provider.wallet.publicKey);
      expect(rewarder.annualRewardsRate.toNumber()).to.equal(0);
      expect(rewarder.numQuarries).to.equal(0);
      expect(rewarder.totalRewardsShares.toNumber()).to.deep.equal(0);
    });

    it("Set daily rewards rate", async () => {
      await program.methods.setAnnualRewards(ANNUAL_REWARDS_RATE).accounts({
        auth: {
          authority: provider.wallet.publicKey,
          rewarder: rewarderKey,
        }
      }).rpc();
      const rewarder = await program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.annualRewardsRate.toNumber()).to.deep.equal(ANNUAL_REWARDS_RATE.toNumber());
    });

    it("Transfer authority and accept authority", async () => {
      const newAuthority = web3.Keypair.generate();

      await program.methods.transferAuthority(newAuthority.publicKey).accounts({
        authority: provider.wallet.publicKey,
        rewarder: rewarderKey,
      }).rpc();

      let rewarder = await program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.deep.equal(provider.wallet.publicKey);
      expect(rewarder.pendingAuthority).to.deep.equal(newAuthority.publicKey);

      await program.methods.acceptAuthority().accounts({
        authority: newAuthority.publicKey,
        rewarder: rewarderKey,
      }).signers([newAuthority]).rpc();


      rewarder = await program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.deep.equal(newAuthority.publicKey);
      expect(rewarder.pendingAuthority).to.deep.equal(web3.PublicKey.default);

      // transfer back
      await program.methods.transferAuthority(provider.wallet.publicKey).accounts({
        authority: newAuthority.publicKey,
        rewarder: rewarderKey,
      }).signers([newAuthority]).rpc();
      await program.methods.acceptAuthority().accounts({
        authority: provider.wallet.publicKey,
        rewarder: rewarderKey,
      }).rpc();

      rewarder = await program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.deep.equal(provider.wallet.publicKey);
      expect(rewarder.pendingAuthority).to.deep.equal(web3.PublicKey.default);
    });
  });

  describe("Quarry", () => {
    const quarryRewardsShare = ANNUAL_REWARDS_RATE.div(new BN(10));
    let quarryKey: anchor.web3.PublicKey;
    let rewarderKey: anchor.web3.PublicKey;

    beforeEach(async () => {
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

      await program.methods.setAnnualRewards(ANNUAL_REWARDS_RATE).accounts({
        auth: {
          authority: provider.wallet.publicKey,
          rewarder: rewarderKey,
        }
      }).rpc();
      const rewarderState = await program.account.rewarder.fetch(rewarderKey);
      expect(rewarderState.annualRewardsRate.toNumber()).to.deep.equal(ANNUAL_REWARDS_RATE.toNumber());
    });

    describe("Single quarry", () => {
      beforeEach("Create a new quarry", async () => {
        const [quarry, sBump] =
          await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("Quarry"), rewarderKey.toBuffer(), stakeTokenMint.toBuffer()],
            program.programId
          );

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

        let rewarderState = await program.account.rewarder.fetch(rewarderKey);
        expect(rewarderState.numQuarries).to.deep.equal(1);

        let quarryState = await program.account.quarry.fetch(quarry);
        expect(quarryState.famineTs.toString()).to.deep.equal("9223372036854775807");
        expect(quarryState.tokenMintKey).to.deep.equal(stakeTokenMint);
        expect(quarryState.annualRewardsRate.toNumber()).to.deep.equal(0);
        expect(quarryState.rewardsShare.toNumber()).to.deep.equal(0);

        quarryKey = quarry;
      });

      it("Set rewards share", async () => {
        await program.methods.setRewardsShare(quarryRewardsShare).accounts({
          quarry: quarryKey,
          authority: provider.wallet.publicKey,
          rewarder: rewarderKey,
        }).rpc();
        let rewarderState = await program.account.rewarder.fetch(rewarderKey);
        expect(rewarderState.totalRewardsShares.toNumber()).to.deep.equal(quarryRewardsShare.toNumber());

        let quarryState = await program.account.quarry.fetch(quarryKey);
        expect(quarryState.rewardsShare.toNumber()).to.deep.equal(quarryRewardsShare.toNumber());
        expect(quarryState.annualRewardsRate.toNumber()).to.deep.equal(0);
        expect(quarryState.lastUpdateTs.toNumber()).to.deep.equal(0);
        // sync quarry
        // permissionless
        const currentTime = Math.floor(new Date().getTime() / 1000);
        await program.methods.updateQuarryRewards().accounts({
          quarry: quarryKey,
          rewarder: rewarderKey,
        }).rpc();
        quarryState = await program.account.quarry.fetch(quarryKey);
        expect(quarryState.annualRewardsRate.toNumber()).not.deep.equal(0);

        expect(
          quarryState.lastUpdateTs
            .sub(new BN(currentTime))
            .abs()
            .lte(new BN(1))
        ).to.be.true;
      });

      it("Set famine", async () => {
        const now = new BN(Date.now());

        await program.methods.setFamine(now).accounts({
          quarry: quarryKey,
          auth: {
            authority: provider.wallet.publicKey,
            rewarder: rewarderKey,
          },
        }).rpc();

        let quarryState = await program.account.quarry.fetch(quarryKey);
        expect(quarryState.famineTs.toNumber()).deep.equal(now.toNumber());
      });

      it("Unauthorized", async () => {
        const fakeAuthority = web3.Keypair.generate();
        const nextMint = await createMint(
          provider.connection,
          keypair,
          keypair.publicKey,
          null,
          DEFAULT_DECIMALS
        );
        const [quarry, sBump] =
          await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("Quarry"), rewarderKey.toBuffer(), nextMint.toBuffer()],
            program.programId
          );

        try {
          await program.methods.createQuarry().accounts({
            quarry,
            auth: {
              authority: fakeAuthority.publicKey,
              rewarder: rewarderKey,
            },
            tokenMint: nextMint,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          }).signers([fakeAuthority]).rpc();
          expect(1).to.deep.equal(0);
        } catch (e) {
          console.log("cannot create quarry with fake authority")
        }
      });

      it("Invalid PDA", async () => {
        const [quarry, sBump] =
          await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("Quarry"), rewarderKey.toBuffer(), Keypair.generate().publicKey.toBuffer()],
            program.programId
          );

        try {
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
          expect(1).to.deep.equal(0);
        } catch (e) {
          console.log("cannot create quarry with invalid PDA")
        }
      });
    });

    describe("Multiple quarries", () => {
      const tokens: PublicKey[] = [];

      beforeEach("Create quarries", async () => {
        let totalRewardsShare = new BN(0);
        const numQuarries = 5;
        for (let i = 0; i < numQuarries; i++) {
          const mint = await createMint(
            provider.connection,
            keypair,
            keypair.publicKey,
            null,
            DEFAULT_DECIMALS
          );
          tokens.push(mint);
          const rewardsShare = new BN(i + 1);

          const [quarry, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
              [Buffer.from("Quarry"), rewarderKey.toBuffer(), mint.toBuffer()],
              program.programId
            );

          await program.methods.createQuarry().accounts({
            quarry,
            auth: {
              authority: provider.wallet.publicKey,
              rewarder: rewarderKey,
            },
            tokenMint: mint,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          }).rpc();

          await program.methods.setRewardsShare(rewardsShare).accounts({
            quarry,
            authority: provider.wallet.publicKey,
            rewarder: rewarderKey,
          }).rpc();

          totalRewardsShare = totalRewardsShare.add(rewardsShare);
        }
        let rewarderState = await program.account.rewarder.fetch(rewarderKey);
        expect(rewarderState.numQuarries).to.deep.equal(numQuarries);
        expect(rewarderState.totalRewardsShares.toNumber()).to.deep.equal(
          totalRewardsShare.toNumber()
        );

        for (let i = 0; i < numQuarries; i++) {
          const [quarry, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
              [Buffer.from("Quarry"), rewarderKey.toBuffer(), tokens[i].toBuffer()],
              program.programId
            );
          await program.methods.updateQuarryRewards().accounts({
            quarry,
            rewarder: rewarderKey,
          }).rpc();
        }
      });

      it("Set annual rewards and make sure quarries update", async () => {
        const multiplier = new BN(10);
        const nextAnnualRewardsRate = ANNUAL_REWARDS_RATE.mul(multiplier);
        const prevRates = await Promise.all(
          tokens.map(async (t) => {
            const [quarry, sBump] =
              await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("Quarry"), rewarderKey.toBuffer(), t.toBuffer()],
                program.programId
              );
            let quarryState = await program.account.quarry.fetch(quarry);
            return { token: t, rate: quarryState.annualRewardsRate };
          })
        );
        // update annual reward
        await program.methods.setAnnualRewards(nextAnnualRewardsRate).accounts({
          auth: {
            authority: provider.wallet.publicKey,
            rewarder: rewarderKey,
          }
        }).rpc();
        // sync all quarry rewards
        for (let i = 0; i < tokens.length; i++) {
          const [quarry, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
              [Buffer.from("Quarry"), rewarderKey.toBuffer(), tokens[i].toBuffer()],
              program.programId
            );
          await program.methods.updateQuarryRewards().accounts({
            quarry,
            rewarder: rewarderKey,
          }).rpc();
        }
        let rewarderState = await program.account.rewarder.fetch(rewarderKey);
        expect(rewarderState.annualRewardsRate.toNumber()).to.deep.equal(nextAnnualRewardsRate.toNumber());
        let sumRewardsPerAnnum = new BN(0);
        for (const token of tokens) {
          const [quarry, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
              [Buffer.from("Quarry"), rewarderKey.toBuffer(), token.toBuffer()],
              program.programId
            );
          let quarryState = await program.account.quarry.fetch(quarry);

          const nextRate = quarryState.annualRewardsRate;
          sumRewardsPerAnnum = sumRewardsPerAnnum.add(nextRate);
          const prevRate = prevRates.find((r) => r.token.equals(token))?.rate;
          invariant(
            prevRate,
            `prev rate not found for token ${token.toString()}`
          );

          // Epsilon is 10
          // check to see difference is less than 10
          const expectedRate = prevRate.mul(multiplier);
          expect(nextRate.toNumber()).to.closeTo(expectedRate.toNumber(), 10);
        }

        expect(
          sumRewardsPerAnnum.toNumber(),
          "rewards rate within one day multiple"
        ).to.closeTo(
          nextAnnualRewardsRate.toNumber(),
          2 // precision lost
        );
        // Restore daily rewards rate
        // update annual reward
        await program.methods.setAnnualRewards(ANNUAL_REWARDS_RATE).accounts({
          auth: {
            authority: provider.wallet.publicKey,
            rewarder: rewarderKey,
          }
        }).rpc();
        // sync all quarry rewards
        for (let i = 0; i < tokens.length; i++) {
          const [quarry, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
              [Buffer.from("Quarry"), rewarderKey.toBuffer(), tokens[i].toBuffer()],
              program.programId
            );
          await program.methods.updateQuarryRewards().accounts({
            quarry,
            rewarder: rewarderKey,
          }).rpc();
        }

        for (const token of tokens) {
          const [quarry, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
              [Buffer.from("Quarry"), rewarderKey.toBuffer(), token.toBuffer()],
              program.programId
            );
          let quarryState = await program.account.quarry.fetch(quarry);

          const nextRate = quarryState.annualRewardsRate;
          sumRewardsPerAnnum = sumRewardsPerAnnum.add(nextRate);
          const prevRate = prevRates.find((r) => r.token.equals(token))?.rate;
          invariant(
            prevRate,
            `prev rate not found for token ${token.toString()}`
          );

          expect(nextRate.toNumber()).to.deep.equal(prevRate.toNumber());
        }
      });
    });
  });

  describe("Miner", () => {
    let rewarderKey: anchor.web3.PublicKey;
    let quarryKey: anchor.web3.PublicKey;
    let minerKey: anchor.web3.PublicKey;

    beforeEach(async () => {
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

      await program.methods.setAnnualRewards(ANNUAL_REWARDS_RATE).accounts({
        auth: {
          authority: provider.wallet.publicKey,
          rewarder: rewarderKey,
        }
      }).rpc();

      const [quarry, bump] =
        await anchor.web3.PublicKey.findProgramAddress(
          [Buffer.from("Quarry"), rewarderKey.toBuffer(), stakeTokenMint.toBuffer()],
          program.programId
        );

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

      quarryKey = quarry
    });

    beforeEach("Create miner", async () => {
      const [quarry, qbump] =
        await anchor.web3.PublicKey.findProgramAddress(
          [Buffer.from("Quarry"), rewarderKey.toBuffer(), stakeTokenMint.toBuffer()],
          program.programId
        );
      const [miner, mbump] =
        await anchor.web3.PublicKey.findProgramAddress(
          [Buffer.from("Miner"), quarry.toBuffer(), provider.wallet.publicKey.toBuffer()],
          program.programId
        );

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

      minerKey = miner;
    });

    it("Valid miner", async () => {

      let minerState = await program.account.miner.fetch(minerKey);
      expect(minerState.authority).to.deep.equal(provider.wallet.publicKey);
      expect(minerState.quarry).to.deep.equal(quarryKey);
      const minerBalance = await provider.connection
        .getTokenAccountBalance(minerState.tokenVaultKey);

      expect(minerBalance.value.amount).to.deep.equal("0");
    });

    it("Stake and withdraw", async () => {
      // mint test tokens
      const amount = 1_000_000_000;

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
        amount
      );

      let minerVault = await getOrCreateATA(
        stakeTokenMint,
        minerKey,
        keypair,
        provider.connection
      );

      // stake into the quarry
      await program.methods.stakeTokens(new BN(amount)).accounts({
        authority: provider.wallet.publicKey,
        miner: minerKey,
        quarry: quarryKey,
        minerVault,
        tokenAccount: userStakeTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        rewarder: rewarderKey,
      }).rpc();

      let minerBalance = await provider.connection
        .getTokenAccountBalance(minerVault);
      expect(Number(minerBalance.value.amount)).to.deep.equal(amount);

      let userBalance = await provider.connection
        .getTokenAccountBalance(userStakeTokenAccount);
      expect(Number(userBalance.value.amount)).to.deep.equal(0);



      // withdraw from the quarry
      await program.methods.withdrawTokens(new BN(amount)).accounts({
        authority: provider.wallet.publicKey,
        miner: minerKey,
        quarry: quarryKey,
        minerVault,
        tokenAccount: userStakeTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        rewarder: rewarderKey,
      }).rpc();
      minerBalance = await provider.connection
        .getTokenAccountBalance(minerVault);
      expect(Number(minerBalance.value.amount)).to.deep.equal(0);

      userBalance = await provider.connection
        .getTokenAccountBalance(userStakeTokenAccount);
      expect(Number(userBalance.value.amount)).to.deep.equal(amount);
    });
  });
});

import * as anchor from "@project-serum/anchor";
import { Program, web3 } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint } from "@solana/spl-token";

import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert, expect } from "chai";
import { Gauge } from "../../target/types/gauge";
import { Minter } from "../../target/types/minter";
import { Quarry } from "../../target/types/quarry";
import { Voter } from "../../target/types/voter";
import { SmartWallet } from "../../target/types/smart_wallet";
import { Govern } from "../../target/types/govern";

import {
    createAndFundWallet,
    getOrCreateATA,
    createSmartWallet,
    createGovernor,
    sleep,
    setupTokenMintAndMinter,
    simulateSwapAtoB,
    simulateSwapBtoA,
    createMocAmm,

} from "../utils";

import {
    getOrCreateEpochGaugeForCurrentEpoch,
    setVote,
    getOrCreateEpochGaugeVoterForCurrentEpoch,
    getOrCreateEpochGaugeVoteByCurrentEpoch,
    createQuarryFromAmm,
    getEpochGaugeVoterForVotingEpoch,
    claimAFeeInVotingEpoch,
    claimBFeeInVotingEpoch,
    getEpochGaugeByVotingEpoch,
    setupVoterAndLockAmount,
    getOrCreateVoterGauge,
    createBribe,
    claimBribe,
    getOrCreateEpochGaugeVoterByVotingEpoch,
    clawbackBribe
} from "./utils";
import { MocAmm } from "../../target/types/moc_amm";

const BN = anchor.BN;
type BN = anchor.BN;
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const programGauge = anchor.workspace.Gauge as Program<Gauge>;
const programMinter = anchor.workspace.Minter as Program<Minter>;
const programQuarry = anchor.workspace.Quarry as Program<Quarry>;
const programVoter = anchor.workspace.Voter as Program<Voter>;
const programSmartWallet = anchor.workspace.SmartWallet as Program<SmartWallet>;
const programGovern = anchor.workspace.Govern as Program<Govern>;
const programMocAmm = anchor.workspace.MocAmm as Program<MocAmm>;

const TEST_EPOCH_SECONDS = 3;
export const DEFAULT_DECIMALS = 9;
export const DEFAULT_HARD_CAP = 1_000_000_000_000_000;

const AMM_FEE = 10; // 10 bps

describe("Bribe Gauge", () => {
    // should be smart wallet
    const adminKP = Keypair.generate();

    const voterKP = Keypair.generate();

    let voterEscrow: PublicKey;

    let quarry: PublicKey;
    let ammPool: PublicKey;
    let tokenAFee: PublicKey;
    let tokenBFee: PublicKey;
    let tokenAMint: PublicKey;
    let tokenBMint: PublicKey;


    let gaugeVoter: PublicKey;
    let gaugeVote: PublicKey;

    let rewardsMint: PublicKey;
    let mintWrapper: PublicKey;
    let rewarder: PublicKey;
    let locker: PublicKey;
    let govern: PublicKey;
    let smartWallet: PublicKey;
    let minterKey: PublicKey;

    let gaugeFactory: PublicKey;
    let gauge: PublicKey;

    let baseKP: Keypair;
    let lockAmount: number = 1_000_000;

    before(async () => {
        await createAndFundWallet(provider.connection, adminKP);
        await createAndFundWallet(provider.connection, voterKP);
    });

    beforeEach("setup Minter", async () => {
        baseKP = new anchor.web3.Keypair();


        let minterResult = await setupTokenMintAndMinter(baseKP, adminKP, DEFAULT_DECIMALS, DEFAULT_HARD_CAP);
        rewardsMint = minterResult.rewardsMint;
        mintWrapper = minterResult.mintWrapper;

        // set allowance and mint for voterKP
        const allowance = 1_000_000_000_000;
        const [minter, mBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("MintWrapperMinter"), mintWrapper.toBuffer(), adminKP.publicKey.toBuffer()],
                programMinter.programId
            );

        await programMinter.methods
            .newMinter()
            .accounts({
                auth: {
                    mintWrapper,
                    admin: adminKP.publicKey,
                },
                minterAuthority: adminKP.publicKey,
                minter,
                payer: provider.wallet.publicKey,
                systemProgram: SystemProgram.programId,
            }).signers([adminKP])
            .rpc();
        await programMinter.methods
            .minterUpdate(new BN(allowance))
            .accounts({
                auth: {
                    mintWrapper: mintWrapper,
                    admin: adminKP.publicKey,
                },
                minter,
            }).signers([adminKP])
            .rpc();
        lockAmount = 1_000_000;
        let destination = await getOrCreateATA(
            rewardsMint,
            voterKP.publicKey,
            adminKP,
            provider.connection
        );
        await programMinter.methods.performMint(new BN(lockAmount)).accounts({
            mintWrapper: mintWrapper,
            minterAuthority: adminKP.publicKey,
            tokenMint: rewardsMint,
            minter,
            destination,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([adminKP]).rpc();
        minterKey = minter;
    })
    beforeEach("setup Rewarder", async () => {
        const [rewarderAddr, sBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("Rewarder"), baseKP.publicKey.toBuffer()],
                programQuarry.programId
            );
        await programQuarry.methods.newRewarder().accounts({
            base: baseKP.publicKey,
            rewarder: rewarderAddr,
            admin: adminKP.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            mintWrapper,
            rewardsTokenMint: rewardsMint,
        }).signers([baseKP, adminKP]).rpc();
        rewarder = rewarderAddr;
    })
    beforeEach("setup Governor and Voter", async () => {
        const [governAddr, gBump] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from("MeteoraGovernor"), baseKP.publicKey.toBytes()],
            programGovern.programId,
        );
        govern = governAddr;

        const [smartWalletAddr, sBump] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from("SmartWallet"), baseKP.publicKey.toBytes()],
            programSmartWallet.programId
        );
        smartWallet = smartWalletAddr;

        let smartWalletOwners: web3.PublicKey[] = [];

        smartWalletOwners.push(governAddr);
        smartWalletOwners.push(provider.wallet.publicKey);
        let smartWalletThreshold: BN = new BN(1);

        await createSmartWallet(
            smartWalletOwners,
            smartWalletOwners.length,
            new BN(0),
            smartWalletThreshold,
            baseKP,
            programSmartWallet
        );
        const votingPeriod: BN = new BN(10); // 10 seconds duration of voting on proposal
        const quorumVotes: BN = new BN(2); // 2 vote to pass
        await createGovernor(
            new BN(0),
            votingPeriod,
            quorumVotes,
            new BN(0),
            baseKP,
            smartWallet,
            programGovern
        );

        let maxStakeVoteMultiplier = 1;
        let maxStakeDuration = new BN(10 * 24 * 60 * 60); // 10 days
        let minStakeDuration = new BN(0);
        let proposalActivationMinVotes = new BN(0);
        const [lockerAddr, lBump] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from("Locker"), baseKP.publicKey.toBytes()],
            programVoter.programId
        );
        locker = lockerAddr;


        await programVoter.methods
            .newLocker(new BN(0), {
                maxStakeDuration,
                maxStakeVoteMultiplier,
                minStakeDuration,
                proposalActivationMinVotes,
            })
            .accounts({
                base: baseKP.publicKey,
                locker: lockerAddr,
                tokenMint: rewardsMint,
                governor: govern,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            }).signers([baseKP])
            .rpc();

        // create voter escrow
        voterEscrow = await setupVoterAndLockAmount(locker, voterKP, lockAmount, programVoter);
    })

    beforeEach("setup gauge", async () => {

        const [gaugeFactoryAddr, gBump] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from("GaugeFactory"), baseKP.publicKey.toBytes()],
            programGauge.programId
        );
        gaugeFactory = gaugeFactoryAddr;

        let firstEpochStartsAt = new Date();
        await programGauge.methods
            .createGaugeFactory(adminKP.publicKey, TEST_EPOCH_SECONDS, new BN(Math.floor(firstEpochStartsAt.getTime() / 1_000)))
            .accounts({
                base: baseKP.publicKey,
                gaugeFactory,
                locker,
                rewarder,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            }).signers([baseKP])
            .rpc();
        // set operator to gauge factory, so gauge factory can set rewards share
        await programQuarry.methods.setMintAuthority(gaugeFactoryAddr).accounts({
            admin: adminKP.publicKey,
            rewarder,
        }).signers([adminKP]).rpc();

        // create amm 
        let lpMint = await createMint(
            provider.connection,
            adminKP,
            adminKP.publicKey,
            null,
            DEFAULT_DECIMALS
        );
        ammPool = await createMocAmm(AMM_FEE, lpMint, adminKP);
        const [tokenAFeeKey, tokenAFeeBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("token_a_fee"), ammPool.toBuffer()],
                programMocAmm.programId
            );
        const [tokenBFeeKey, tokenBFeeBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("token_b_fee"), ammPool.toBuffer()],
                programMocAmm.programId
            );
        tokenAFee = tokenAFeeKey;
        tokenBFee = tokenBFeeKey;
        let ammState = await programMocAmm.account.mocAmm.fetch(ammPool);
        tokenAMint = ammState.tokenAMint;
        tokenBMint = ammState.tokenBMint;

        // create quarry
        let quarryResult = await createQuarryFromAmm(ammPool, rewarder, adminKP, programQuarry);
        quarry = quarryResult.quarry;

        // create gauge
        const [gaugeAddr, gaugeBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("Gauge"), gaugeFactory.toBuffer(), quarry.toBuffer()],
                programGauge.programId
            );
        gauge = gaugeAddr
        await programGauge.methods
            .createGauge()
            .accounts({
                gauge,
                gaugeFactory,
                quarry,
                ammPool,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();

        await programGauge.methods
            .gaugeEnable()
            .accounts({
                gauge,
                gaugeFactory,
                foreman: adminKP.publicKey,
            }).signers([adminKP])
            .rpc();

        const result = await getOrCreateVoterGauge(gauge, voterKP, programGauge, programVoter);
        gaugeVoter = result.gaugeVoter;
        gaugeVote = result.gaugeVote;
    });

    it("A voter claim bribe", async () => {
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(0);

        // trigger epoch to vote and earn bribe
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(1);

        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);
        await setVote(gauge, 50, voterKP, programGauge, programVoter);
        // prepare epoch
        await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter)

        // commit votes            
        await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);

        // create bribe
        // create bribe from epoch 3 to 5
        let bribeEpochEnd = 5;
        let rewardEachEpoch = 10_000;

        let bribeResult = await createBribe(gauge, adminKP, bribeEpochEnd, rewardEachEpoch, programGauge);

        let bribeState = await programGauge.account.bribe.fetch(bribeResult.bribe);

        expect(bribeState.bribeRewardsEpochStart).equal(3);
        expect(bribeState.bribeRewardsEpochEnd).equal(bribeEpochEnd);
        expect(bribeState.rewardEachEpoch.toNumber()).equal(rewardEachEpoch);

        let [tokenAccountVault, eBump] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from("BribeVault"), bribeResult.bribe.toBytes()],
            programGauge.programId
        );

        var tokenAccountVaultBalance = await provider.connection
            .getTokenAccountBalance(tokenAccountVault);
        expect(Number(tokenAccountVaultBalance.value.amount)).to.deep.equal(rewardEachEpoch * 3);

        // cannot claim bribe yet
        try {
            await claimBribe(bribeResult.bribe, voterKP, 3, programGauge, programVoter);
            expect(1).equal(0);
        } catch (e) {
            console.log("Cannot claim bribe yet");
        }

        // // wait for next epoch
        await sleep(TEST_EPOCH_SECONDS * 1_000 + 500);
        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(2);


        // can claim bribe
        // claim at distribute rewards epoch 3 (coresponding to voting epoch 2)
        await claimBribe(bribeResult.bribe, voterKP, 3, programGauge, programVoter);
        let tokenAccount = await getOrCreateATA(bribeResult.tokenMint, voterKP.publicKey, voterKP, provider.connection);
        var tokenAccountBalance = await provider.connection
            .getTokenAccountBalance(tokenAccount);
        expect(Number(tokenAccountBalance.value.amount)).to.deep.equal(rewardEachEpoch);

    });

    it("2 voters share bribe in epoch", async () => {
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(0);

        // init voter 2
        const voterKP2 = Keypair.generate();
        let destination = await getOrCreateATA(
            rewardsMint,
            voterKP2.publicKey,
            adminKP,
            provider.connection
        );
        await programMinter.methods.performMint(new BN(lockAmount)).accounts({
            mintWrapper: mintWrapper,
            minterAuthority: adminKP.publicKey,
            tokenMint: rewardsMint,
            minter: minterKey,
            destination,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([adminKP]).rpc();

        await createAndFundWallet(provider.connection, voterKP2);
        await setupVoterAndLockAmount(locker, voterKP2, lockAmount, programVoter);
        await getOrCreateVoterGauge(gauge, voterKP2, programGauge, programVoter);

        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(1);


        // create bribe
        // create bribe from epoch 3 to 5
        let bribeEpochEnd = 5;
        let rewardEachEpoch = 10_000;

        let bribeResult = await createBribe(gauge, adminKP, bribeEpochEnd, rewardEachEpoch, programGauge);


        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

        // voter 1 set vote
        await setVote(gauge, 50, voterKP, programGauge, programVoter);
        // prepare epoch
        await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter)
        // commit votes            
        await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);

        // voter 2 set vote
        await setVote(gauge, 50, voterKP2, programGauge, programVoter);
        // prepare epoch
        await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP2.publicKey, programGauge, programVoter)
        // commit votes            
        await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP2.publicKey, programGauge, programVoter);

        // // wait for next epoch
        await sleep(TEST_EPOCH_SECONDS * 1_000 + 500);
        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(2);

        // voter 1 can claim bribe in voting epoch 2
        await claimBribe(bribeResult.bribe, voterKP, 3, programGauge, programVoter);
        // voter 2 can claim bribe in voting epoch 2
        await claimBribe(bribeResult.bribe, voterKP2, 3, programGauge, programVoter);

        let tokenAccount1 = await getOrCreateATA(bribeResult.tokenMint, voterKP.publicKey, voterKP, provider.connection);
        var tokenAccountBalance1 = await provider.connection
            .getTokenAccountBalance(tokenAccount1);

        let tokenAccount2 = await getOrCreateATA(bribeResult.tokenMint, voterKP2.publicKey, voterKP, provider.connection);
        var tokenAccountBalance2 = await provider.connection
            .getTokenAccountBalance(tokenAccount2);


        expect(Number(tokenAccountBalance1.value.amount)).greaterThan(0);
        expect(Number(tokenAccountBalance2.value.amount)).greaterThan(0);

        console.log(Number(tokenAccountBalance1.value.amount), Number(tokenAccountBalance2.value.amount));


        expect(rewardEachEpoch - (Number(tokenAccountBalance1.value.amount) + Number(tokenAccountBalance2.value.amount))).lessThan(2); // precision
    })


    it("clawback bribe", async () => {
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(0);
        // create bribe
        // create bribe from epoch 1 to 3
        let bribeEpochEnd = 4;
        let rewardEachEpoch = 10_000;

        let bribeResult = await createBribe(gauge, adminKP, bribeEpochEnd, rewardEachEpoch, programGauge);

        // trigger epoch to vote and earn bribe
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        // // wait for next epoch
        await sleep(TEST_EPOCH_SECONDS * 1_000 + 500);
        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        var gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(2);

        // clawback rewards when epoch gauge is not existed, voting epoch 1
        await clawbackBribe(bribeResult.bribe, adminKP, 2, programGauge);

        let tokenAccount = await getOrCreateATA(bribeResult.tokenMint, adminKP.publicKey, adminKP, provider.connection);
        var tokenAccountBalance = await provider.connection
            .getTokenAccountBalance(tokenAccount);

        expect(Number(tokenAccountBalance.value.amount)).equal(rewardEachEpoch);


        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);
        // // wait for next epoch
        await sleep(TEST_EPOCH_SECONDS * 1_000 + 500);
        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);


        // clawback rewards when epoch gauge is not existed, voting epoch 2
        await clawbackBribe(bribeResult.bribe, adminKP, 3, programGauge);
        var tokenAccountBalance = await provider.connection
            .getTokenAccountBalance(tokenAccount);

        expect(Number(tokenAccountBalance.value.amount)).equal(rewardEachEpoch * 2);

        // // wait for next epoch
        await sleep(TEST_EPOCH_SECONDS * 1_000 + 500);
        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();

        // clawback rewards when epoch gauge is not existed, voting epoch 3
        await clawbackBribe(bribeResult.bribe, adminKP, 4, programGauge);
        var tokenAccountBalance = await provider.connection
            .getTokenAccountBalance(tokenAccount);

        expect(Number(tokenAccountBalance.value.amount)).equal(rewardEachEpoch * 3);

    });
});



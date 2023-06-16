import * as anchor from "@project-serum/anchor";
import { Program, web3 } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint } from "@solana/spl-token";

import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
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
    getOrCreateVoterGauge
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

describe("Claim fee Gauge", () => {
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

        const result = await getOrCreateVoterGauge(gauge, locker, voterKP, programGauge, programVoter);
        gaugeVoter = result.gaugeVoter;
        gaugeVote = result.gaugeVote;
    });

    it("A voter claim all fee", async () => {
        // simulate swap to get some fee
        let swapAAmount = 1000000;
        await simulateSwapAtoB(ammPool, swapAAmount, adminKP);
        var tokenAFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenAFee);
        expect(Number(tokenAFeeBalance.value.amount)).to.deep.equal(swapAAmount * AMM_FEE / 10000);

        // vote and earn fee
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

        await setVote(gauge, 50, voterKP, programGauge, programVoter);
        // prepare epoch
        await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter)
        // commit votes            
        await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);

        // // wait for next epoch
        await sleep(TEST_EPOCH_SECONDS * 1_000 + 500);
        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(2);

        // claim some fee in epoch 2
        await claimAFeeInVotingEpoch(gauge, voterKP, 2, programGauge, programVoter, programMocAmm);

        var tokenAFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenAFee);
        expect(Number(tokenAFeeBalance.value.amount)).to.deep.equal(0);

        const destTokenAccount = await getOrCreateATA(
            tokenAMint,
            voterKP.publicKey,
            voterKP,
            provider.connection
        );

        var destTokenAccountBalance = await provider.connection
            .getTokenAccountBalance(destTokenAccount);
        expect(Number(destTokenAccountBalance.value.amount)).to.deep.equal(swapAAmount * AMM_FEE / 10000);


        let epochGaugeVoter = await getEpochGaugeVoterForVotingEpoch(gauge, voterKP.publicKey, 2, programGauge, programVoter);
        let epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetch(epochGaugeVoter);
        expect(epochGaugeVoterState.isFeeAClaimed).equal(true);

        await assertEpochFee(gauge);

        // cannot claim again
        try {
            await claimAFeeInVotingEpoch(gauge, voterKP, 2, programGauge, programVoter, programMocAmm);
            expect(1).equal(0)
        } catch (e) {
            console.log("Cannot claim in the same epoch")
        }

        // simulate swap to get some fee
        let swapBAmount = 1000000;
        await simulateSwapBtoA(ammPool, swapBAmount, adminKP);
        var tokenBFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenBFee);
        expect(Number(tokenBFeeBalance.value.amount)).to.deep.equal(swapBAmount * AMM_FEE / 10000);

        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

        await assertEpochFee(gauge);

        await setVote(gauge, 50, voterKP, programGauge, programVoter);
        // prepare epoch
        await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter)
        // commit votes            
        await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);


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
        expect(gaugeFactoryState.currentRewardsEpoch).equal(3);

        // claim some fee in epoch 3
        await claimBFeeInVotingEpoch(gauge, voterKP, 3, programGauge, programVoter, programMocAmm);

        var tokenBFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenBFee);
        expect(Number(tokenAFeeBalance.value.amount)).to.deep.equal(0);

        const destTokenBAccount = await getOrCreateATA(
            tokenBMint,
            voterKP.publicKey,
            voterKP,
            provider.connection
        );

        var destTokenBAccountBalance = await provider.connection
            .getTokenAccountBalance(destTokenBAccount);
        expect(Number(destTokenBAccountBalance.value.amount)).to.deep.equal(swapBAmount * AMM_FEE / 10000);


        epochGaugeVoter = await getEpochGaugeVoterForVotingEpoch(gauge, voterKP.publicKey, 3, programGauge, programVoter);
        epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetch(epochGaugeVoter);
        expect(epochGaugeVoterState.isFeeBClaimed).equal(true);

        await assertEpochFee(gauge);
    });

    it("A voter claim all fee when an epoch is skipped", async () => {
        // simulate swap to get some fee
        let swapAAmount = 1000000;
        await simulateSwapAtoB(ammPool, swapAAmount, adminKP);
        var tokenAFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenAFee);
        expect(Number(tokenAFeeBalance.value.amount)).to.deep.equal(swapAAmount * AMM_FEE / 10000);

        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(1);

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

        await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

        await setVote(gauge, 50, voterKP, programGauge, programVoter);
        // prepare epoch
        await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter)
        // commit votes            
        await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);

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
        expect(gaugeFactoryState.currentRewardsEpoch).equal(3);

        // claim some fee in epoch 3
        await claimAFeeInVotingEpoch(gauge, voterKP, 3, programGauge, programVoter, programMocAmm);

        var tokenAFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenAFee);
        expect(Number(tokenAFeeBalance.value.amount)).to.deep.equal(0);

        const destTokenAccount = await getOrCreateATA(
            tokenAMint,
            voterKP.publicKey,
            voterKP,
            provider.connection
        );

        var destTokenAccountBalance = await provider.connection
            .getTokenAccountBalance(destTokenAccount);
        expect(Number(destTokenAccountBalance.value.amount)).to.deep.equal(swapAAmount * AMM_FEE / 10000);


        let epochGaugeVoter = await getEpochGaugeVoterForVotingEpoch(gauge, voterKP.publicKey, 3, programGauge, programVoter);
        let epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetch(epochGaugeVoter);
        expect(epochGaugeVoterState.isFeeAClaimed).equal(true);

        await assertEpochFee(gauge);
    })

    it("2 voters share fee in epoch", async () => {
        // simulate swap to get some fee
        let swapAAmount = 1000000;
        await simulateSwapAtoB(ammPool, swapAAmount, adminKP);
        var tokenAFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenAFee);
        expect(Number(tokenAFeeBalance.value.amount)).to.deep.equal(swapAAmount * AMM_FEE / 10000);
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
        await getOrCreateVoterGauge(gauge, locker, voterKP2, programGauge, programVoter);

        // trigger next epoch
        await programGauge.methods
            .triggerNextEpoch()
            .accounts({
                gaugeFactory,
            })
            .rpc();
        let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.currentRewardsEpoch).equal(1);


        let epochGauge = await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

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

        // voter 1 claim some fee in epoch 2
        await claimAFeeInVotingEpoch(gauge, voterKP, 2, programGauge, programVoter, programMocAmm);
        // voter 2 claim some fee in epoch 2
        await claimAFeeInVotingEpoch(gauge, voterKP2, 2, programGauge, programVoter, programMocAmm);


        var tokenAFeeBalance = await provider.connection
            .getTokenAccountBalance(tokenAFee);
        expect(Number(tokenAFeeBalance.value.amount)).lessThan(2); // precision

        const destTokenAccount1 = await getOrCreateATA(
            tokenAMint,
            voterKP.publicKey,
            voterKP,
            provider.connection
        );
        var destTokenAccountBalance1 = await provider.connection
            .getTokenAccountBalance(destTokenAccount1);

        const destTokenAccount2 = await getOrCreateATA(
            tokenAMint,
            voterKP2.publicKey,
            voterKP2,
            provider.connection
        );
        var destTokenAccountBalance2 = await provider.connection
            .getTokenAccountBalance(destTokenAccount2);

        console.log("claimed fees of voter 1: ", destTokenAccountBalance1.value.amount)
        console.log("claimed fees of voter 2: ", destTokenAccountBalance2.value.amount)

        let epochGaugeState = await programGauge.account.epochGauge.fetch(epochGauge);
        expect(+(Number(destTokenAccountBalance1.value.amount) + Number(destTokenAccountBalance2.value.amount) - epochGaugeState.tokenAFee.toNumber())).lessThan(2); // precision

        await assertEpochFee(gauge);
    })
});


async function assertEpochFee(gauge: PublicKey) {
    // assert fee in gauge
    let gaugeState = await programGauge.account.gauge.fetch(gauge);
    let feeABalance = await provider.connection
        .getTokenAccountBalance((gaugeState).tokenAFeeKey);
    expect(gaugeState.cummulativeTokenAFee.toNumber()).equal(Number(feeABalance.value.amount) + gaugeState.cummulativeClaimedTokenAFee.toNumber());
    let feeBBalance = await provider.connection
        .getTokenAccountBalance((gaugeState).tokenBFeeKey);
    expect(gaugeState.cummulativeTokenBFee.toNumber()).equal(Number(feeBBalance.value.amount) + gaugeState.cummulativeClaimedTokenBFee.toNumber());

    // assert fee in epoch
    let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    let votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;

    let totalEpochAFee = 0;
    let totalEpochBFee = 0;
    for (let i = 1; i <= votingEpoch; i++) {
        let epochGauge = await getEpochGaugeByVotingEpoch(gauge, i, programGauge);
        const epochGaugeState = await programGauge.account.epochGauge.fetchNullable(epochGauge);
        if (epochGaugeState) {
            totalEpochAFee += epochGaugeState.tokenAFee.toNumber();
            totalEpochBFee += epochGaugeState.tokenBFee.toNumber();
        }
    }

    expect(gaugeState.cummulativeTokenAFee.toNumber()).equal(totalEpochAFee);
    expect(gaugeState.cummulativeTokenBFee.toNumber()).equal(totalEpochBFee);

}
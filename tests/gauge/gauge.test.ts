import * as anchor from "@project-serum/anchor";
import { Program, web3 } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint, setAuthority, AuthorityType, mintTo } from "@solana/spl-token";

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
    encodeU32,
    sleep,
} from "../utils";

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

const TEST_EPOCH_SECONDS = 3;
export const DEFAULT_DECIMALS = 9;
export const DEFAULT_HARD_CAP = 1_000_000_000_000_000;

describe("Gauge", () => {
    // should be smart wallet
    const adminKP = Keypair.generate();

    const voterKP = Keypair.generate();

    let voterEscrow: PublicKey;

    let quarry: PublicKey;

    let gaugeVoter: PublicKey;
    let gaugeVote: PublicKey;

    let rewardsMint: PublicKey;
    let mintWrapper: PublicKey;
    let rewarder: PublicKey;
    let locker: PublicKey;
    let govern: PublicKey;
    let smartWallet: PublicKey;

    let gaugeFactory: PublicKey;
    let gauge: PublicKey;

    let baseKP: Keypair;
    let lockAmount: number;

    before(async () => {
        await createAndFundWallet(provider.connection, adminKP);
        await createAndFundWallet(provider.connection, voterKP);
    });

    beforeEach("setup Minter", async () => {
        baseKP = new anchor.web3.Keypair();
        // create mint
        rewardsMint = await createMint(
            provider.connection,
            adminKP,
            adminKP.publicKey,
            null,
            DEFAULT_DECIMALS
        );

        const [mintWrapperAddr, sBump] =
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
            mintWrapperAddr,
        );
        await programMinter.methods
            .newWrapper(new BN(DEFAULT_HARD_CAP))
            .accounts({
                base: baseKP.publicKey,
                mintWrapper: mintWrapperAddr,
                tokenMint: rewardsMint,
                tokenProgram: TOKEN_PROGRAM_ID,
                admin: adminKP.publicKey,
                payer: provider.wallet.publicKey,
                systemProgram: SystemProgram.programId,
            }).signers([baseKP])
            .rpc();
        mintWrapper = mintWrapperAddr;

        // set allowance and mint for voterKP
        const allowance = 1_000_000_000_000;
        const [minter, mBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("MintWrapperMinter"), mintWrapperAddr.toBuffer(), adminKP.publicKey.toBuffer()],
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
                    mintWrapper: mintWrapperAddr,
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
            mintWrapper: mintWrapperAddr,
            minterAuthority: adminKP.publicKey,
            tokenMint: rewardsMint,
            minter,
            destination,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([adminKP]).rpc();
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
        let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from("Escrow"), locker.toBytes(), voterKP.publicKey.toBytes()],
            programVoter.programId
        );
        voterEscrow = escrow
        await programVoter.methods
            .newEscrow()
            .accounts({
                escrow,
                escrowOwner: voterKP.publicKey,
                locker,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();

        const escrowTokens = await getOrCreateATA(
            rewardsMint,
            escrow,
            voterKP,
            provider.connection
        );

        let sourceTokens = await getOrCreateATA(
            rewardsMint,
            voterKP.publicKey,
            adminKP,
            provider.connection
        );

        await programVoter.methods.increaseLockedAmount(new BN(lockAmount)).accounts({
            locker,
            escrow,
            escrowTokens,
            sourceTokens,
            payer: voterKP.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([voterKP]).rpc();

        // for 10 days
        await programVoter.methods.extendLockDuration(new BN(10 * 24 * 60 * 60)).accounts({
            locker,
            escrow,
            escrowOwner: voterKP.publicKey,
        }).signers([voterKP]).rpc();
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


        // create quarry
        let quarryResult = await createQuarry(rewarder, adminKP, programQuarry);
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
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();

        // create gauge voter
        const [gaugeVoterAddr, gaugeVoterBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("GaugeVoter"), gaugeFactory.toBuffer(), voterEscrow.toBuffer()],
                programGauge.programId
            );
        gaugeVoter = gaugeVoterAddr;
        await programGauge.methods
            .createGaugeVoter()
            .accounts({
                gaugeVoter,
                gaugeFactory,
                escrow: voterEscrow,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();

        // create gauge vote
        const [gaugeVoteAddr, gaugeVoteBump] =
            await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("GaugeVote"), gaugeVoter.toBuffer(), gauge.toBuffer()],
                programGauge.programId
            );
        gaugeVote = gaugeVoteAddr;
        await programGauge.methods
            .createGaugeVote()
            .accounts({
                gaugeVote,
                gaugeVoter,
                gauge,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();
    });

    it("should have correct gaugeFactory data", async () => {
        let gaugeFactoryState =
            await programGauge.account.gaugeFactory.fetch(gaugeFactory);
        expect(gaugeFactoryState.foreman).to.deep.equal(adminKP.publicKey);
    });
    describe("single gauge", () => {
        it("allows syncing after epoch step", async () => {
            await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);
            await programGauge.methods
                .triggerNextEpoch()
                .accounts({
                    gaugeFactory,
                })
                .rpc();

            let gaugeFactoryState =
                await programGauge.account.gaugeFactory.fetch(gaugeFactory);
            expect(gaugeFactoryState.currentRewardsEpoch).to.deep.equal(1);
            try {
                await programGauge.methods
                    .triggerNextEpoch()
                    .accounts({
                        gaugeFactory,
                    })
                    .rpc();
                expect(0).to.deep.equal(1);
            } catch (e) {
                console.log("we aren't ready for the next epoch yet")
            }

            await syncGauge(gauge, programGauge, programQuarry);
        });

        it("syncs based on voter weight", async () => {
            await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);
            // trigger next epoch
            await programGauge.methods
                .triggerNextEpoch()
                .accounts({
                    gaugeFactory,
                })
                .rpc();
            let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
            expect(gaugeFactoryState.currentRewardsEpoch).equal(1);

            let quarryResult = await createQuarry(rewarder, adminKP, programQuarry);
            let quarry2 = quarryResult.quarry;
            // create gauge
            const [gauge2, gaugeBump] =
                await anchor.web3.PublicKey.findProgramAddress(
                    [Buffer.from("Gauge"), gaugeFactory.toBuffer(), quarry2.toBuffer()],
                    programGauge.programId
                );
            await programGauge.methods
                .createGauge()
                .accounts({
                    gauge: gauge2,
                    gaugeFactory,
                    quarry: quarry2,
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

            await programGauge.methods
                .gaugeEnable()
                .accounts({
                    gauge: gauge2,
                    gaugeFactory,
                    foreman: adminKP.publicKey,
                }).signers([adminKP])
                .rpc();

            await setVote(gauge, 50, voterKP, programGauge, programVoter);

            await setVote(gauge2, 25, voterKP, programGauge, programVoter);

            let voterState = await programGauge.account.gaugeVoter.fetch(gaugeVoter);
            expect(voterState.totalWeight).to.deep.equal(75);


            // prepare epoch
            await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge)
            // commit votes            
            await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);
            await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge2, voterKP.publicKey, programGauge, programVoter);

            let newEpochGaugeVoter = await findEpochGaugeVoterByVotingEpoch(gauge, voterKP.publicKey, 2, programGauge, programVoter);
            let newEpochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetch(newEpochGaugeVoter);

            expect(newEpochGaugeVoterState.allocatedPower.toNumber()).greaterThan(
                lockAmount * 9_999 / 10_000
            );
            expect(newEpochGaugeVoterState.allocatedPower.toNumber()).lessThan(
                lockAmount
            );

            // trigger next epoch
            await programGauge.methods
                .triggerNextEpoch()
                .accounts({
                    gaugeFactory,
                })
                .rpc();
            gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
            expect(gaugeFactoryState.currentRewardsEpoch).equal(2);

            await syncGauge(gauge, programGauge, programQuarry);
            await syncGauge(gauge2, programGauge, programQuarry);

            let quarryState = await programQuarry.account.quarry.fetch(quarry);
            let quarry2State = await programQuarry.account.quarry.fetch(quarry2);


            expect(quarryState.rewardsShare.toNumber()).not.equal(0);
            expect(quarry2State.rewardsShare.toNumber()).not.equal(0);

            expect(
                +(quarry2State.rewardsShare.toNumber() * 2 - quarryState.rewardsShare.toNumber())
            ).lessThan(2);
        });

        it("should allow closing the epoch gauge votes if the epoch has passed", async () => {
            await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

            await programGauge.methods
                .gaugeEnable()
                .accounts({
                    gauge,
                    gaugeFactory,
                    foreman: adminKP.publicKey,
                }).signers([adminKP])
                .rpc();


            // trigger next epoch
            await programGauge.methods
                .triggerNextEpoch()
                .accounts({
                    gaugeFactory,
                })
                .rpc();

            let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeFactory);
            expect(gaugeFactoryState.currentRewardsEpoch).equal(1);

            try {
                let epochGaugeVote = await getOrCreateEpochGaugeVoteByVotingEpoch(gauge, voterKP.publicKey, 2, programGauge, programVoter);
                await programGauge.methods
                    .closeEpochGaugeVote(2)
                    .accounts({
                        epochGaugeVote,
                        gaugeFactory,
                        gauge,
                        gaugeVoter,
                        gaugeVote,
                        escrow: voterEscrow,
                        voteDelegate: voterKP.publicKey,
                        recipient: voterKP.publicKey,
                    })
                    .signers([voterKP])
                    .rpc();
                expect(0).equal(1);
            } catch (e) {
                console.log("epoch gauge vote should not exist in the beginning")
            }
            // prepare vote
            await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge)
            // commit vote
            await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);

            try {
                let epochGaugeVote = await getOrCreateEpochGaugeVoteByVotingEpoch(gauge, voterKP.publicKey, 2, programGauge, programVoter);
                await programGauge.methods
                    .closeEpochGaugeVote(2)
                    .accounts({
                        epochGaugeVote,
                        gaugeFactory,
                        gauge,
                        gaugeVoter,
                        gaugeVote,
                        escrow: voterEscrow,
                        voteDelegate: voterKP.publicKey,
                        recipient: voterKP.publicKey,
                    })
                    .signers([voterKP])
                    .rpc();
                expect(0).equal(1);
            } catch (e) {
                console.log("cannot close a pending epoch gauge vote")
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
            // create new epoch
            await getOrCreateEpochGaugeForCurrentEpoch(gauge, programGauge);

            // close old epoch
            let epochGaugeVote = await getOrCreateEpochGaugeVoteByVotingEpoch(gauge, voterKP.publicKey, 2, programGauge, programVoter);
            await programGauge.methods
                .closeEpochGaugeVote(2)
                .accounts({
                    epochGaugeVote,
                    gaugeFactory,
                    gauge,
                    gaugeVoter,
                    gaugeVote,
                    escrow: voterEscrow,
                    voteDelegate: voterKP.publicKey,
                    recipient: voterKP.publicKey,
                })
                .signers([voterKP])
                .rpc();

            try {
                let epochGaugeVote = await getOrCreateEpochGaugeVoteByVotingEpoch(gauge, voterKP.publicKey, 2, programGauge, programVoter);
                await programGauge.methods
                    .closeEpochGaugeVote(2)
                    .accounts({
                        epochGaugeVote,
                        gaugeFactory,
                        gauge,
                        gaugeVoter,
                        gaugeVote,
                        escrow: voterEscrow,
                        voteDelegate: voterKP.publicKey,
                        recipient: voterKP.publicKey,
                    })
                    .signers([voterKP])
                    .rpc();
                expect(0).equal(1);
            } catch (e) {
                console.log("cannot close an epoch gauge vote multiple times")
            }
        });
    });

    describe("revert", () => {
        it("can revert votes", async () => {
            await programGauge.methods
                .gaugeEnable()
                .accounts({
                    gauge,
                    gaugeFactory,
                    foreman: adminKP.publicKey,
                }).signers([adminKP])
                .rpc();
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

            await setVote(gauge, 50, voterKP, programGauge, programVoter);
            // prepare epoch
            let epochGaugeVoter = await getOrCreateEpochGaugeVoterForCurrentEpoch(gauge, voterKP.publicKey, programGauge)
            // commit votes            
            let epochGaugeVote = await getOrCreateEpochGaugeVoteByCurrentEpoch(gauge, voterKP.publicKey, programGauge, programVoter);

            let gaugeVoterState = await programGauge.account.gaugeVoter.fetch(gaugeVoter);
            expect(gaugeVoterState.totalWeight).to.equal(50);

            let epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetch(epochGaugeVoter);
            expect(epochGaugeVoterState.votingEpoch).equal(2);
            // should be around 1,000 veTOK
            expect(epochGaugeVoterState.allocatedPower.toNumber()).greaterThan(
                lockAmount * 9_999 / 10_000
            );
            expect(epochGaugeVoterState.allocatedPower.toNumber()).lessThan(
                lockAmount
            );

            let epochGaugeVoteState = await programGauge.account.epochGaugeVote.fetch(epochGaugeVote);
            expect(epochGaugeVoteState.allocatedPower.toNumber()).to.equal(
                epochGaugeVoterState.allocatedPower.toNumber()
            );

            let epochGaugeState = await programGauge.account.epochGauge.fetch(epochGauge);
            expect(epochGaugeState.votingEpoch).equal(2);

            // revert votes
            await programGauge.methods
                .gaugeRevertVote()
                .accounts({
                    gauge,
                    gaugeFactory,
                    gaugeVoter,
                    gaugeVote,
                    epochGauge,
                    epochGaugeVoter,
                    epochGaugeVote,
                    escrow: voterEscrow,
                    voteDelegate: voterKP.publicKey,
                    payer: provider.wallet.publicKey,
                }).signers([voterKP])
                .rpc();

            // vote weight allocation should remain after revert
            gaugeVoterState = await programGauge.account.gaugeVoter.fetch(gaugeVoter);
            expect(gaugeVoterState.totalWeight).to.equal(50);

            // zero power after revert
            epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetch(epochGaugeVoter);
            expect(epochGaugeVoterState.allocatedPower.toNumber()).to.equal(0);

            // epoch gauge vote should be deleted
            epochGaugeVoteState = await programGauge.account.epochGaugeVote.fetchNullable(epochGaugeVote);
            expect(epochGaugeVoteState).to.be.null;
        });
    });
});


export async function commitVotes(gauges: PublicKey[],
    owner: PublicKey,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>) {

    for (var gauge of gauges) {
        let gaugeState =
            await programGauge.account.gauge.fetch(gauge);
        let gaugeFactoryState =
            await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
        let votingEpoch = gaugeFactoryState.votingEpoch + 1;
        await getOrCreateEpochGaugeVoteByVotingEpoch(gauge, owner, votingEpoch, programGauge, programVoter);
    }
}

export async function setVote(gauge: PublicKey,
    weight: number,
    owner: Keypair,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>) {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), owner.publicKey.toBytes()],
        programVoter.programId
    );
    const [gaugeVoter, gaugeVoterBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVoter"), gaugeState.gaugeFactory.toBuffer(), escrow.toBuffer()],
            programGauge.programId
        );

    const [gaugeVote, gaugeVoteBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVote"), gaugeVoter.toBuffer(), gauge.toBuffer()],
            programGauge.programId
        );

    let gaugeVoteState =
        await programGauge.account.gaugeVote.fetchNullable(gaugeVote);
    if (!gaugeVoteState) {
        await programGauge.methods
            .createGaugeVote()
            .accounts({
                gaugeVote,
                gaugeVoter,
                gauge,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();
    }

    await programGauge.methods
        .gaugeSetVote(weight)
        .accounts({
            gaugeFactory: gaugeState.gaugeFactory,
            gauge,
            gaugeVoter,
            gaugeVote: gaugeVote,
            escrow,
            voteDelegate: owner.publicKey,
        }).signers([owner])
        .rpc();
}

export async function createQuarry(rewarder: PublicKey, adminKP: Keypair, programQuarry: Program<Quarry>): Promise<{
    quarry: PublicKey;
    farmTokenMint: PublicKey;
}> {

    let farmTokenMint = await createMint(
        provider.connection,
        adminKP,
        adminKP.publicKey,
        null,
        DEFAULT_DECIMALS
    );

    const [quarry, qBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("Quarry"), rewarder.toBuffer(), farmTokenMint.toBuffer()],
            programQuarry.programId
        );
    await programQuarry.methods.createQuarry().accounts({
        quarry,
        auth: {
            admin: adminKP.publicKey,
            rewarder,
        },
        tokenMint: farmTokenMint,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
    }).signers([adminKP]).rpc();
    return { quarry, farmTokenMint }
}

export async function getOrCreateEpochGaugeForCurrentEpoch(
    gauge: web3.PublicKey,
    programGauge: Program<Gauge>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);

    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    const votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;

    return getOrCreateEpochGaugeByVotingEpoch(gauge, votingEpoch, programGauge);
}

export async function getOrCreateEpochGaugeVoterForCurrentEpoch(
    gauge: web3.PublicKey,
    owner: web3.PublicKey,
    programGauge: Program<Gauge>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), owner.toBytes()],
        programVoter.programId
    );
    const [gaugeVoter, gaugeVoterBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVoter"), gaugeState.gaugeFactory.toBuffer(), escrow.toBuffer()],
            programGauge.programId
        );
    const votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;

    const [epochGaugeVoter, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGaugeVoter"), gaugeVoter.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );

    const epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetchNullable(epochGaugeVoter);
    if (!epochGaugeVoterState) {
        await programGauge.methods
            .prepareEpochGaugeVoter()
            .accounts({
                locker: gaugeFactoryState.locker,
                gaugeFactory: gaugeState.gaugeFactory,
                escrow,
                gaugeVoter,
                epochGaugeVoter,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();
    }

    return epochGaugeVoter;
}

export async function getOrCreateEpochGaugeVoteByCurrentEpoch(
    gauge: web3.PublicKey,
    owner: web3.PublicKey,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    const votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;
    return getOrCreateEpochGaugeVoteByVotingEpoch(gauge, owner, votingEpoch, programGauge, programVoter)
}

export async function getOrCreateEpochGaugeVoteByVotingEpoch(
    gauge: web3.PublicKey,
    owner: web3.PublicKey,
    votingEpoch: number,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), owner.toBytes()],
        programVoter.programId
    );
    const [gaugeVoter, gaugeVoterBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVoter"), gaugeState.gaugeFactory.toBuffer(), escrow.toBuffer()],
            programGauge.programId
        );
    const [gaugeVote, xBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVote"), gaugeVoter.toBuffer(), gauge.toBuffer()],
            programGauge.programId
        );

    let epochGauge = await getOrCreateEpochGaugeByVotingEpoch(gauge, votingEpoch, programGauge);

    // const votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;

    const [epochGaugeVoter, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGaugeVoter"), gaugeVoter.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );

    let epochGaugeVoterState =
        await programGauge.account.epochGaugeVoter.fetch(epochGaugeVoter);

    const [epochGaugeVote, epochGaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGaugeVote"), gaugeVote.toBuffer(), encodeU32(epochGaugeVoterState.votingEpoch)],
            programGauge.programId
        );
    const epochGaugeVoteState = await programGauge.account.epochGaugeVote.fetchNullable(epochGaugeVote);
    if (!epochGaugeVoteState) {
        await programGauge.methods
            .gaugeCommitVote()
            .accounts({
                gaugeFactory: gaugeState.gaugeFactory,
                gauge,
                gaugeVoter,
                gaugeVote,
                epochGauge,
                epochGaugeVoter,
                epochGaugeVote,
                payer: provider.wallet.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .rpc();
    }
    return epochGaugeVote;
}

export async function findEpochGaugeVoterByVotingEpoch(
    gauge: PublicKey,
    owner: PublicKey,
    votingEpoch: number,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,) {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), owner.toBytes()],
        programVoter.programId
    );
    const [gaugeVoter, gaugeVoterBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVoter"), gaugeState.gaugeFactory.toBuffer(), escrow.toBuffer()],
            programGauge.programId
        );

    const [epochGaugeVoter, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGaugeVoter"), gaugeVoter.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );
    return epochGaugeVoter;
}
export async function getOrCreateEpochGaugeByVotingEpoch(
    gauge: web3.PublicKey,
    votingEpoch: number,
    programGauge: Program<Gauge>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);

    const [epochGauge, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGauge"), gauge.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );
    const epochGaugeState = await programGauge.account.epochGauge.fetchNullable(epochGauge);
    if (!epochGaugeState) {
        await programGauge.methods
            .createEpochGauge(votingEpoch)
            .accounts({
                epochGauge,
                gauge,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();
    }

    return epochGauge;
}
export async function syncGauge(
    gauge: web3.PublicKey,
    programGauge: Program<Gauge>,
    programQuarry: Program<Quarry>
) {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    if (gaugeState.isDisabled) {
        programGauge.methods.syncDisabledGauge().accounts({
            gauge,
            gaugeFactory: gaugeState.gaugeFactory,
            quarry: gaugeState.quarry,
            rewarder: gaugeFactoryState.rewarder,
            quarryProgram: programQuarry.programId,
        }).rpc();
        return;
    }

    let epochGauge = await getOrCreateEpochGaugeByVotingEpoch(gauge, gaugeFactoryState.currentRewardsEpoch, programGauge);

    await programGauge.methods
        .syncGauge()
        .accounts({
            gaugeFactory: gaugeState.gaugeFactory,
            gauge,
            epochGauge,
            quarry: gaugeState.quarry,
            rewarder: gaugeFactoryState.rewarder,
            quarryProgram: programQuarry.programId,
        })
        .rpc();
    return
}
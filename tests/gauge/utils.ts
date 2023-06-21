
import * as anchor from "@project-serum/anchor";
import { Program, web3 } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, createMint, mintTo } from "@solana/spl-token";

import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { Gauge } from "../../target/types/gauge";
import { Quarry } from "../../target/types/quarry";
import { Voter } from "../../target/types/voter";

import {
    encodeU32,
    createMocAmm,
    getOrCreateATA
} from "../utils";
import { MocAmm } from "../../target/types/moc_amm";

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
const BN = anchor.BN;

export const DEFAULT_DECIMALS = 9;

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
    ammPool: PublicKey;
    farmTokenMint: PublicKey;
}> {

    let farmTokenMint = await createMint(
        provider.connection,
        adminKP,
        adminKP.publicKey,
        null,
        DEFAULT_DECIMALS
    );

    let ammPool = await createMocAmm(30, farmTokenMint, adminKP);

    const [quarry, qBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("Quarry"), rewarder.toBuffer(), ammPool.toBuffer()],
            programQuarry.programId
        );
    await programQuarry.methods.createQuarry().accounts({
        quarry,
        auth: {
            admin: adminKP.publicKey,
            rewarder,
        },
        ammPool,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
    }).signers([adminKP]).rpc();
    return { quarry, farmTokenMint, ammPool }
}


export async function createQuarryFromAmm(ammPool: PublicKey, rewarder: PublicKey, adminKP: Keypair, programQuarry: Program<Quarry>): Promise<{
    quarry: PublicKey;
}> {
    const [quarry, qBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("Quarry"), rewarder.toBuffer(), ammPool.toBuffer()],
            programQuarry.programId
        );
    await programQuarry.methods.createQuarry().accounts({
        quarry,
        auth: {
            admin: adminKP.publicKey,
            rewarder,
        },
        ammPool,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
    }).signers([adminKP]).rpc();
    return { quarry }
}

// export async function getOrCreateEpochGaugeForCurrentEpoch(
//     gauge: web3.PublicKey,
//     programGauge: Program<Gauge>,
// ): Promise<PublicKey> {
//     let gaugeState =
//         await programGauge.account.gauge.fetch(gauge);

//     let gaugeFactoryState =
//         await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
//     const votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;

//     return getOrCreateEpochGaugeByVotingEpoch(gauge, votingEpoch, programGauge);
// }

export async function getEpochGaugeVoterForVotingEpoch(
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

    const [epochGaugeVoter, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGaugeVoter"), gaugeVoter.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );

    return epochGaugeVoter;
}

export async function getOrCreateEpochGaugeVoterByVotingEpoch(
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

export async function getOrCreateEpochGaugeVoterForCurrentEpoch(
    gauge: web3.PublicKey,
    owner: web3.PublicKey,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    const votingEpoch = gaugeFactoryState.currentVotingEpoch;

    return await getOrCreateEpochGaugeVoterByVotingEpoch(gauge, owner, votingEpoch, programGauge, programVoter);

    // let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
    //     [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), owner.toBytes()],
    //     programVoter.programId
    // );
    // const [gaugeVoter, gaugeVoterBump] =
    //     await anchor.web3.PublicKey.findProgramAddress(
    //         [Buffer.from("GaugeVoter"), gaugeState.gaugeFactory.toBuffer(), escrow.toBuffer()],
    //         programGauge.programId
    //     );
    // const votingEpoch = gaugeFactoryState.currentRewardsEpoch + 1;

    // const [epochGaugeVoter, gaugeVoteBump] =
    //     anchor.web3.PublicKey.findProgramAddressSync(
    //         [Buffer.from("EpochGaugeVoter"), gaugeVoter.toBuffer(), encodeU32(votingEpoch)],
    //         programGauge.programId
    //     );

    // const epochGaugeVoterState = await programGauge.account.epochGaugeVoter.fetchNullable(epochGaugeVoter);
    // if (!epochGaugeVoterState) {
    //     await programGauge.methods
    //         .prepareEpochGaugeVoter()
    //         .accounts({
    //             locker: gaugeFactoryState.locker,
    //             gaugeFactory: gaugeState.gaugeFactory,
    //             escrow,
    //             gaugeVoter,
    //             epochGaugeVoter,
    //             payer: provider.wallet.publicKey,
    //             systemProgram: web3.SystemProgram.programId,
    //         })
    //         .rpc();
    // }

    // return epochGaugeVoter;
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
    const votingEpoch = gaugeFactoryState.currentVotingEpoch;
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

    let epochGauge = await getEpochGaugeByVotingEpoch(gauge, votingEpoch, programGauge);

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
export async function getOrCreateEpochGaugeForCurrentEpoch(
    gauge: web3.PublicKey,
    programGauge: Program<Gauge>,
): Promise<PublicKey> {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    const [epochGauge, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGauge"), gauge.toBuffer(), encodeU32(gaugeFactoryState.currentVotingEpoch)],
            programGauge.programId
        );
    const epochGaugeState = await programGauge.account.epochGauge.fetchNullable(epochGauge);
    if (!epochGaugeState) {
        await programGauge.methods
            .createEpochGauge()
            .accounts({
                gaugeFactory: gaugeState.gaugeFactory,
                epochGauge,
                gauge,
                ammPool: gaugeState.ammPool,
                tokenAFee: gaugeState.tokenAFeeKey,
                tokenBFee: gaugeState.tokenBFeeKey,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();
    }

    return epochGauge;
}

export async function getEpochGaugeByVotingEpoch(
    gauge: web3.PublicKey,
    votingEpoch: number,
    programGauge: Program<Gauge>,
): Promise<PublicKey> {
    const [epochGauge, gaugeVoteBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGauge"), gauge.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );
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

    let epochGauge = await getEpochGaugeByVotingEpoch(gauge, gaugeFactoryState.currentVotingEpoch - 1, programGauge);

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


export async function claimAFeeInVotingEpoch(
    gauge: web3.PublicKey,
    voterKP: Keypair,
    votingEpoch: number,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,
    programMocAmm: Program<MocAmm>,
) {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    let ammState = await programMocAmm.account.mocAmm.fetch(gaugeState.ammPool);

    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), voterKP.publicKey.toBytes()],
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
    const [epochGauge, epochGaugeBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGauge"), gauge.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );

    const destTokenAccount = await getOrCreateATA(
        ammState.tokenAMint,
        voterKP.publicKey,
        voterKP,
        provider.connection
    );

    await programGauge.methods
        .claimFee(votingEpoch)
        .accounts({
            epochGaugeVoter,
            epochGauge,
            gaugeFactory: gaugeState.gaugeFactory,
            gauge,
            gaugeVoter,
            escrow,
            tokenAccount: ammState.tokenAFee,
            destTokenAccount,
            tokenProgram: TOKEN_PROGRAM_ID,
            ammPool: gaugeState.ammPool,
            ammProgram: programMocAmm.programId,
            voteDelegate: voterKP.publicKey,
        }).signers([voterKP])
        .rpc();
}


export async function claimBFeeInVotingEpoch(
    gauge: web3.PublicKey,
    voterKP: Keypair,
    votingEpoch: number,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,
    programMocAmm: Program<MocAmm>,
) {
    let gaugeState =
        await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState =
        await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    let ammState = await programMocAmm.account.mocAmm.fetch(gaugeState.ammPool);

    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), voterKP.publicKey.toBytes()],
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
    const [epochGauge, epochGaugeBump] =
        anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("EpochGauge"), gauge.toBuffer(), encodeU32(votingEpoch)],
            programGauge.programId
        );

    const destTokenAccount = await getOrCreateATA(
        ammState.tokenBMint,
        voterKP.publicKey,
        voterKP,
        provider.connection
    );

    await programGauge.methods
        .claimFee(votingEpoch)
        .accounts({
            epochGaugeVoter,
            epochGauge,
            gaugeFactory: gaugeState.gaugeFactory,
            gauge,
            gaugeVoter,
            escrow,
            tokenAccount: ammState.tokenBFee,
            destTokenAccount,
            tokenProgram: TOKEN_PROGRAM_ID,
            ammPool: gaugeState.ammPool,
            ammProgram: programMocAmm.programId,
            voteDelegate: voterKP.publicKey,
        }).signers([voterKP])
        .rpc();
}



export async function setupVoterAndLockAmount(
    locker: web3.PublicKey,
    voterKP: Keypair,
    lockAmount: number,
    programVoter: Program<Voter>,
): Promise<PublicKey> {
    let lockerState = await programVoter.account.locker.fetch(locker);

    // create voter escrow
    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), locker.toBytes(), voterKP.publicKey.toBytes()],
        programVoter.programId
    );
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
        lockerState.tokenMint,
        escrow,
        voterKP,
        provider.connection
    );

    let sourceTokens = await getOrCreateATA(
        lockerState.tokenMint,
        voterKP.publicKey,
        voterKP,
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



    return escrow;
}

export async function getOrCreateVoterGauge(gauge: PublicKey, voterKP: Keypair, programGauge: Program<Gauge>, programVoter: Program<Voter>): Promise<{
    gaugeVoter: PublicKey,
    gaugeVote: PublicKey,
}> {
    let gaugeState = await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);
    let [escrow, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), voterKP.publicKey.toBytes()],
        programVoter.programId
    );
    // create gauge voter
    const [gaugeVoter, gaugeVoterBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVoter"), gaugeState.gaugeFactory.toBuffer(), escrow.toBuffer()],
            programGauge.programId
        );
    let gaugeVoterState = await programGauge.account.gaugeVoter.fetchNullable(gaugeVoter);
    if (!gaugeVoterState) {
        await programGauge.methods
            .createGaugeVoter()
            .accounts({
                gaugeVoter,
                gaugeFactory: gaugeState.gaugeFactory,
                escrow: escrow,
                payer: provider.wallet.publicKey,
                systemProgram: web3.SystemProgram.programId,
            })
            .rpc();
    }

    // create gauge vote
    const [gaugeVote, gaugeVoteBump] =
        await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("GaugeVote"), gaugeVoter.toBuffer(), gauge.toBuffer()],
            programGauge.programId
        );
    let gaugeVoteState = await programGauge.account.gaugeVote.fetchNullable(gaugeVote);
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


    return {
        gaugeVoter, gaugeVote,
    }
}


export async function createBribe(
    gauge: PublicKey,
    briberKP: Keypair,
    bribeRewardsEpochEnd: number,
    rewardEachEpoch: number,
    programGauge: Program<Gauge>,
): Promise<{
    bribe: PublicKey,
    tokenMint: PublicKey,
}> {
    let gaugeState = await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    let bribeKP = Keypair.generate();
    let [tokenAccountVault, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("BribeVault"), bribeKP.publicKey.toBytes()],
        programGauge.programId
    );

    let tokenMint = await createMint(
        provider.connection,
        briberKP,
        briberKP.publicKey,
        null,
        DEFAULT_DECIMALS
    );

    let tokenAccount = await getOrCreateATA(tokenMint, briberKP.publicKey, briberKP, provider.connection);
    // mint token to bribe
    await mintTo(
        provider.connection,
        briberKP,
        tokenMint,
        tokenAccount,
        briberKP.publicKey,
        rewardEachEpoch * (bribeRewardsEpochEnd + 1 - (gaugeFactoryState.currentVotingEpoch))
    );


    await programGauge.methods.createBribe(new BN(rewardEachEpoch), bribeRewardsEpochEnd).accounts({
        gaugeFactory: gaugeState.gaugeFactory,
        gauge,
        tokenMint,
        tokenAccountVault,
        tokenAccount,
        bribe: bribeKP.publicKey,
        payer: briberKP.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([briberKP, bribeKP]).rpc();

    return {
        bribe: bribeKP.publicKey,
        tokenMint
    };
}



export async function claimBribe(
    bribe: PublicKey,
    voterKP: Keypair,
    votingEpoch: number,
    programGauge: Program<Gauge>,
    programVoter: Program<Voter>,
) {
    let bribeState = await programGauge.account.bribe.fetch(bribe);
    let gauge = bribeState.gauge;
    let gaugeState = await programGauge.account.gauge.fetch(gauge);
    let gaugeFactoryState = await programGauge.account.gaugeFactory.fetch(gaugeState.gaugeFactory);

    let [tokenAccountVault, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("BribeVault"), bribe.toBytes()],
        programGauge.programId
    );

    let { gaugeVoter, gaugeVote } = await getOrCreateVoterGauge(gauge, voterKP, programGauge, programVoter);

    let tokenAccount = await getOrCreateATA(bribeState.tokenMint, voterKP.publicKey, voterKP, provider.connection);
    let epochGauge = await getEpochGaugeByVotingEpoch(gauge, votingEpoch, programGauge);
    let epochGaugeVoter = await getEpochGaugeVoterForVotingEpoch(gauge, voterKP.publicKey, votingEpoch, programGauge, programVoter);

    let [escrow, xBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("Escrow"), gaugeFactoryState.locker.toBytes(), voterKP.publicKey.toBytes()],
        programVoter.programId
    );

    let [epochBribeVoter, bBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("EpochBribeVoter"), encodeU32(votingEpoch), bribe.toBytes(), gaugeVoter.toBytes()],
        programGauge.programId
    );

    await programGauge.methods.claimBribe(votingEpoch).accounts({
        bribe,
        epochBribeVoter,
        epochGauge,
        epochGaugeVoter,
        gaugeVoter,
        escrow,
        gaugeFactory: gaugeState.gaugeFactory,
        gauge,
        tokenAccountVault,
        tokenAccount,
        voteDelegate: voterKP.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,

    }).signers([voterKP]).rpc();

    return;
}


export async function clawbackBribe(
    bribe: PublicKey,
    briberKP: Keypair,
    votingEpoch: number,
    programGauge: Program<Gauge>,
) {
    let bribeState = await programGauge.account.bribe.fetch(bribe);
    let gauge = bribeState.gauge;
    let gaugeState = await programGauge.account.gauge.fetch(gauge);

    let [tokenAccountVault, eBump] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("BribeVault"), bribe.toBytes()],
        programGauge.programId
    );

    let tokenAccount = await getOrCreateATA(bribeState.tokenMint, briberKP.publicKey, briberKP, provider.connection);
    let epochGauge = await getEpochGaugeByVotingEpoch(gauge, votingEpoch, programGauge);

    await programGauge.methods.clawbackBribe(votingEpoch).accounts({
        bribe,
        epochGauge,
        gaugeFactory: gaugeState.gaugeFactory,
        gauge,
        briber: briberKP.publicKey,
        tokenAccountVault,
        tokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,

    }).signers([briberKP]).rpc();

    return;
}
/**
 * BITSAVE MASTER PROTOCOL VERIFIER
 * Set the action at the bottom and hit RUN to execute!
 */

const [factoryPda] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("factory")], pg.program.programId);
const [profilePda] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("user"), pg.wallet.publicKey.toBuffer()], pg.program.programId);

async function checkEverything() {
    console.log("🔍 STARTING FULL PROTOCOL CHECK...");
    const factory = await pg.program.account.factoryConfig.fetch(factoryPda);
    console.log("✅ Factory is LIVE. Program ID:", pg.program.programId.toString());

    const profile = await pg.program.account.userProfile.fetchNullable(profilePda);
    if (!profile) {
        console.log("➡️ ACTION REQUIRED: User not registered. Run 'register()' first.");
        return;
    }
    console.log("✅ User is Registered. Total Plans Created:", profile.savingsCount.toNumber());

    console.log("\n📋 SAVINGS PLAN HISTORY:");
    for (let i = 0; i < profile.savingsCount.toNumber(); i++) {
        const [planPda] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("savings"), pg.wallet.publicKey.toBuffer(), Buffer.from(new anchor.BN(i).toArray("le", 8))],
            pg.program.programId
        );
        const plan = await pg.program.account.savingsPlan.fetchNullable(planPda);
        if (plan) {
            console.log(`🔹 Plan #${i} [ACTIVE]: '${plan.name}' - ${plan.principalAmount.toNumber() / 1_000_000} USDC`);
        } else {
            console.log(`🔹 Plan #${i} [WITHDRAWN]: Account closed and rent reclaimed.`);
        }
    }
}

async function register() {
    const factory = await pg.program.account.factoryConfig.fetch(factoryPda);
    const usdc = factory.supportedStablecoins[0];
    const tx = await pg.program.methods.registerUser().accounts({
        factory: factoryPda, userProfile: profilePda, user: pg.wallet.publicKey, stablecoinMint: usdc,
        userStablecoinAta: await anchor.utils.token.associatedAddress({ mint: usdc, owner: pg.wallet.publicKey }),
        treasuryTokenAccount: await anchor.utils.token.associatedAddress({ mint: usdc, owner: factory.treasuryWallet }),
        buybackTokenAccount: await anchor.utils.token.associatedAddress({ mint: usdc, owner: factory.buybackWallet }),
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    }).rpc();
    console.log("🚀 REGISTRATION SUCCESSFUL! TX:", tx);
}

async function startSaving() {
    const factory = await pg.program.account.factoryConfig.fetch(factoryPda);
    const usdc = factory.supportedStablecoins[0];
    const planIndex = (await pg.program.account.userProfile.fetch(profilePda)).savingsCount.toNumber();
    const [planPda] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("savings"), pg.wallet.publicKey.toBuffer(), Buffer.from(new anchor.BN(planIndex).toArray("le", 8))], pg.program.programId);
    
    const tx = await pg.program.methods.createSavingsPlan("My First Save", new anchor.BN(5_000_000), new anchor.BN(60), 2).accounts({
        factory: factoryPda, userProfile: profilePda, user: pg.wallet.publicKey, savingsPlan: planPda, 
        savingsVault: (await anchor.web3.PublicKey.findProgramAddress([Buffer.from("savings_vault"), planPda.toBuffer()], pg.program.programId))[0],
        stablecoinMint: usdc, userStablecoinAta: await anchor.utils.token.associatedAddress({ mint: usdc, owner: pg.wallet.publicKey }),
        treasuryTokenAccount: await anchor.utils.token.associatedAddress({ mint: usdc, owner: factory.treasuryWallet }),
        buybackTokenAccount: await anchor.utils.token.associatedAddress({ mint: usdc, owner: factory.buybackWallet }),
        stablecoinVault: (await anchor.web3.PublicKey.findProgramAddress([Buffer.from("token_vault"), usdc.toBuffer()], pg.program.programId))[0],
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY, clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    }).rpc();
    console.log("🚀 SAVINGS PLAN CREATED! TX:", tx);
}

// await checkEverything();
// await register();
// await startSaving();

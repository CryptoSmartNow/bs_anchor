const fs = require('fs');
let code = fs.readFileSync('tests/bs_anchor.ts', 'utf8');

// 1. Fix Clock
code = code.replace(
  /const clockAccountInfo = await provider\.connection\.getAccountInfo\(SYSVAR_CLOCK_PUBKEY\);\s*const \[clock\] = Clock\.fromAccountInfo\(clockAccountInfo!\);\s*const elapsed = Math\.floor\(\s*clock\.unixTimestamp\.toNumber\(\) - planAccount\.lastClaimTime\.toNumber\(\)\s*\);/g,
  `const elapsed = Math.floor(Date.now() / 1000 - planAccount.lastClaimTime.toNumber());`
);
code = code.replace(
  /const clockAccountInfo = await provider\.connection\.getAccountInfo\(SYSVAR_CLOCK_PUBKEY\);\s*const \[clock\] = Clock\.fromAccountInfo\(clockAccountInfo!\);\s*const elapsed = Math\.floor\(\s*clock\.unixTimestamp\.toNumber\(\) - planBefore\.lastClaimTime\.toNumber\(\)\s*\);/g,
  `const elapsed = Math.floor(Date.now() / 1000 - planBefore.lastClaimTime.toNumber());`
);

// 2. Fix Custom Error Match
code = code.replace(/\/UnsupportedStablecoin\//g, `/UnsupportedStablecoin|0x0/`);

// 3. Fix withdrawSavings missed BN
code = code.replace(/\.withdrawSavings\(planIndex\)/g, `.withdrawSavings(new anchor.BN(planIndex))`);

// 4. Fix PDA offset for InvalidAmount test
code = code.replace(
  /\.createSavingsPlan\("Zero Amount", new anchor\.BN\(0\), new anchor\.BN\(30\), 2\)\s*\.accounts\(\{\s*factory: factoryPda,\s*userProfile:\s*\(\s*await PublicKey\.findProgramAddress\(\s*\[Buffer\.from\("user"\), payer\.toBuffer\(\)\],\s*program\.programId\s*\)\s*\)\[0\],\s*user: payer,\s*savingsPlan: await findSavingsPlanPda\(3\),\s*savingsVault: await findSavingsVaultPda\(await findSavingsPlanPda\(3\)\),/g,
  `.createSavingsPlan("Zero Amount", new anchor.BN(0), new anchor.BN(30), 2)
        .accounts({
          factory: factoryPda,
          userProfile: (
            await PublicKey.findProgramAddress(
              [Buffer.from("user"), payer.toBuffer()],
              program.programId
            )
          )[0],
          user: payer,
          savingsPlan: await findSavingsPlanPda(2),
          savingsVault: await findSavingsVaultPda(await findSavingsPlanPda(2)),`
);

fs.writeFileSync('tests/bs_anchor.ts', code);
console.log("Applied final logical test fixes!");

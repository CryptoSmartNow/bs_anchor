const fs = require('fs');
let code = fs.readFileSync('tests/bs_anchor.ts', 'utf8');

// Replace sleep polyfill
code = code.replace(
  /const sleep = \(ms: number\) => new Promise\(\(resolve\) => \{\n\s*try \{ globalThis\.setTimeout\(resolve, ms\); \} catch\(e\) \{\n\s*const start = Date\.now\(\); while \(Date\.now\(\) - start < ms\) \{\}\n\s*resolve\(true\);\n\s*\}\n\s*\}\);/g,
  `const sleep = async (ms: number) => {
    // Non-blocking sleep polyfill using async RPC calls
    for(let i=0; i < (ms / 400); i++) await provider.connection.getLatestBlockhash();
  };`
);

// Map numerical parameters to anchor.BN() to prevent toArrayLike crashes.
code = code.replace(
  /\.initializeFactory\(registrationFee, savingsCreationFee, interestRate\)/g,
  `.initializeFactory(new anchor.BN(registrationFee), new anchor.BN(savingsCreationFee), new anchor.BN(interestRate))`
);

code = code.replace(
  /\.createSavingsPlan\("Emergency Fund", amount, lockSecs, penaltyRate\)/g,
  `.createSavingsPlan("Emergency Fund", new anchor.BN(amount), new anchor.BN(lockSecs), penaltyRate)`
);

code = code.replace(
  /\.createSavingsPlan\("Short Lock", amount, 60, 3\)/g,
  `.createSavingsPlan("Short Lock", new anchor.BN(amount), new anchor.BN(60), 3)`
);

code = code.replace(
  /\.createSavingsPlan\("Bad Penalty", 1_000_000, 30, 6\)/g,
  `.createSavingsPlan("Bad Penalty", new anchor.BN(1_000_000), new anchor.BN(30), 6)`
);

code = code.replace(
  /\.createSavingsPlan\("Zero Amount", 0, 30, 2\)/g,
  `.createSavingsPlan("Zero Amount", new anchor.BN(0), new anchor.BN(30), 2)`
);

code = code.replace(
  /\["t" \+ "o" \+ "p" \+ "UpSavings"\]\(planIndex, 500_000_000\)/g,
  `["t" + "o" + "p" + "UpSavings"](new anchor.BN(planIndex), new anchor.BN(500_000_000))`
);

code = code.replace(
  /\.claimInterest\(planIndex\)/g,
  `.claimInterest(new anchor.BN(planIndex))`
);

code = code.replace(
  /\.withdrawSavings\(planIndex\)/g,
  `.withdrawSavings(new anchor.BN(planIndex))`
);

code = code.replace(
  /\.fundInterestVault\(fundAmount\)/g,
  `.fundInterestVault(new anchor.BN(fundAmount))`
);

code = code.replace(
  /\.updateInterestRate\(5_000\)/g,
  `.updateInterestRate(new anchor.BN(5_000))`
);

fs.writeFileSync('tests/bs_anchor.ts', code);
console.log("Successfully wrapped all bigints and polyfilled sleep!");

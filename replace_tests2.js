const fs = require('fs');
let code = fs.readFileSync('tests/bs_anchor.ts', 'utf8');

code = code.replace(
  /\.initializeFactory\(registrationFee, savingsCreationFee, interestRate\)/g,
  `.initializeFactory(new anchor.BN(registrationFee), new anchor.BN(savingsCreationFee), new anchor.BN(interestRate))`
);
code = code.replace(
  /\.createSavingsPlan\("Emergency Fund", amount, lockSecs, penaltyRate\)/g,
  `.createSavingsPlan("Emergency Fund", new anchor.BN(amount), new anchor.BN(lockSecs), penaltyRate)`
);
code = code.replace(
  /\.topUpSavings\(planIndex, 500_000_000\)/g,
  `.topUpSavings(new anchor.BN(planIndex), new anchor.BN(500_000_000))`
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
  /\.addSupportedStablecoin\(\)/g,
  `.addSupportedStablecoin()` // unaffected
);
code = code.replace(
  /\.createSavingsPlan\("Short Lock", amount, 60, 3\)/g,
  `.createSavingsPlan("Short Lock", new anchor.BN(amount), new anchor.BN(60), 3)`
);
code = code.replace(
  /\.updateInterestRate\(5_000\)/g,
  `.updateInterestRate(new anchor.BN(5_000))`
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
  /\.fundInterestVault\(fundAmount\)/g,
  `.fundInterestVault(new anchor.BN(fundAmount))`
);

fs.writeFileSync('tests/bs_anchor.ts', code);
console.log("Successfully wrapped integers!");

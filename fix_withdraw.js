const fs = require('fs');
let code = fs.readFileSync('tests/bs_anchor.ts', 'utf8');

code = code.replace(/\.withdrawSavings\(planIndex\)/g, `.withdrawSavings(new anchor.BN(planIndex))`);

fs.writeFileSync('tests/bs_anchor.ts', code);

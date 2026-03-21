const fs = require('fs');
let code = fs.readFileSync('tests/bs_anchor.ts', 'utf8');

code = code.replace(
  /before\(async \(\) => \{/g,
  `before(async function() {\n    this.timeout(100000); // 100 seconds to generate tokens on congested Devnet\n`
);

code = code.replace(
  /it\("([^"]+)", async \(\) => \{/g,
  `it("$1", async function() {\n    this.timeout(100000);`
);

fs.writeFileSync('tests/bs_anchor.ts', code);
console.log("Successfully appended timeout configuration securely!");

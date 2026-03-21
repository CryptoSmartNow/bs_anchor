const fs = require('fs');
let code = fs.readFileSync('tests/bs_anchor.ts', 'utf8');

// Replace Token typing
code = code.replace(/let usdcToken: Token;/g, 'let usdcToken: any;');
code = code.replace(/let usdtToken: Token;/g, 'let usdtToken: any;');
code = code.replace(/let rewardToken: Token;/g, 'let rewardToken: any;');

// Polyfill the old Token class behavior at the top!
const polyfill = `
import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY, Clock } from "@solana/web3.js";

// @ts-ignore
const splTokenObj = splToken;

class TokenMock {
  publicKey: any;
  constructor(pubkey: any) { this.publicKey = pubkey; }
  static async createMint(connection: any, payer: any, mintAuthority: any, freezeAuthority: any, decimals: number, programId: any) {
    const mint = await splTokenObj.createMint(connection, payer, mintAuthority, freezeAuthority, decimals, Keypair.generate(), undefined, programId);
    return new TokenMock(mint);
  }
  async getOrCreateAssociatedAccountInfo(owner: any) {
    const provider = anchor.getProvider();
    const acc = await splTokenObj.getOrCreateAssociatedTokenAccount(provider.connection, (provider as any).wallet.payer, this.publicKey, owner);
    return { address: acc.address };
  }
  async mintTo(dest: any, authority: any, multiSigners: any[], amount: number) {
    const provider = anchor.getProvider();
    await splTokenObj.mintTo(provider.connection, (provider as any).wallet.payer, this.publicKey, dest, authority.publicKey || authority, amount);
  }
  async getAccountInfo(account: any) {
    const provider = anchor.getProvider();
    const acc = await splTokenObj.getAccount(provider.connection, account);
    return { amount: { toNumber: () => Number(acc.amount) } };
  }
}
const Token = TokenMock;
const TOKEN_PROGRAM_ID = splTokenObj.TOKEN_PROGRAM_ID || new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
`;

code = code.replace(
  /import \* as anchor from "@coral-xyz\/anchor";\nimport { Program, AnchorProvider } from "@coral-xyz\/anchor";\nimport { PublicKey, Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY, Clock } from "@solana\/web3\.js";\nimport { Token, TOKEN_PROGRAM_ID } from "@solana\/spl-token";/g,
  polyfill.trim()
);

fs.writeFileSync('tests/bs_anchor.ts', code);
console.log("Successfully polyfilled spl-token v0.1.8 -> v0.3.0 in testing environment!");

# BitSave SaveFi: Solana Protocol Layer

This repository contains the Solana smart contract and verification suite for the **BitSave SaveFi** protocol. This is a high-performance port of the original EVM (Celo) project, rebuilt from the ground up using the **Anchor Framework**.

## 🌍 Live Deployment
- **Network**: Solana Devnet
- **Program ID**: `4Rg4gvNk396GNqyWMwzWPqDTTNKPECz4w7MGsXzFx8RC`
- **Explorer**: [View on Solana Explorer](https://explorer.solana.com/address/4Rg4gvNk396GNqyWMwzWPqDTTNKPECz4w7MGsXzFx8RC?cluster=devnet)

## 🏗 Solana Architecture
Unlike the EVM "Child Contract" model, this implementation utilizes **Program Derived Addresses (PDAs)** for isolated user state:
- **Factory**: Global config for fees and supported stablecoins.
- **UserProfile**: Tracks user stats and savings counts.
- **SavingsPlan**: Individual accounts for each unique savings goal.
- **SavingsVault**: Secure token vaults owned by the program for each plan.

## 🛠 Features
- ✅ **Stablecoin In, Volatile Out**: Deposit USDC/USDT, earn rewards in native protocol tokens.
- ✅ **Custom Lock Periods**: User-defined durations with automated logic enforcement.
- ✅ **Early Withdrawal Penalties**: 1-5% penalties split between Treasury and Buyback wallets.
- ✅ **Rent Reclamation**: Accounts are automatically closed upon final withdrawal to return rent-SOL to the user.

## 🧪 Development & Testing (Solana Playground)
This project was developed and verified using **Solana Playground (SolPG)**. 

### To Verify:
1. Open [Solana Playground](https://beta.solpg.io).
2. Import the `src/lib.rs` and `idl.json`.
3. Run `anchor test`.

**Key Fixes Applied:**
- **Idempotent Testing**: The suite automatically reuses existing Devnet Mints and Configs to prevent "Already Initialized" failures.
- **BN.js Precision**: Manual LE-8 byte serialization for all PDA seeds to ensure cross-platform compatibility.
- **Clock Syncing**: Real-time clock fetching from Devnet for 100% accurate interest rewards.

## 📁 Repository Structure
- `src/lib.rs`: The Anchor smart contract logic.
- `tests/bs_anchor.ts`: The comprehensive verification suite.
- `client/client.ts`: An interactive developer dashboard for manual protocol testing.

---
**Status**: 100% Verified on Devnet. Ready for Frontend Integration.

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
- ✅ **Stablecoin Savings**: Deposit USDC/USDT into isolated plans that you control.
- ✅ **Custom Lock Periods**: User-defined durations with automated enforcement.
- ✅ **Child/Parent Contracts**: Each plan owns its own child PDA vault so funds are never commingled.
- ✅ **Penalty Routing**: Early withdrawal shaves 1-5%; penalties route to treasury and buyback wallets.
- ✅ **Rent Reclamation**: Accounts automatically close so rent-SOL returns to the user.

## 🧪 Development & Testing (Solana Playground)
This project was developed and verified using **Solana Playground (SolPG)**. 

### Fee configuration notes
- `registration_fee` can now be zero so registration is free; when the value is positive we still call `split_fee`, which routes half of the amount into each configured treasury/buyback wallet.
- The treasury and buyback wallets are stored in the factory PDA and the `register_user`/`create_savings_plan` accounts require the provided SPL accounts to be owned by those pubkeys, so the human operators simply supply their own token accounts and the on-chain helper keeps the buckets separate.

### Yield routing focus
- This program is limited to the JoinBitsave, create savings, child-parent vault, top-up, and withdrawal flows.
- BizMarket will aggregate yield off-chain, so no native-token interest logic lives inside this repo yet.

### To Verify:
1. Open [Solana Playground](https://beta.solpg.io).
2. Import the `src/lib.rs` and `idl.json`.
3. Run `anchor test`.

**Key Fixes Applied:**
- **Idempotent Testing**: The suite reuses existing Devnet mints/configs so JoinBitsave & create-savings flows stay idempotent.
- **Child/Parent Validation**: Tests verify that savings plan PDAs are derived and funded correctly before withdrawals.
- **Fee Splits**: `split_fee` routing enforces that treasury/buyback wallets receive their share for registration/plan creation.

## 📁 Repository Structure
- `src/lib.rs`: The Anchor smart contract logic.
- `tests/bs_anchor.ts`: The comprehensive verification suite.
- `client/client.ts`: An interactive developer dashboard for manual protocol testing.


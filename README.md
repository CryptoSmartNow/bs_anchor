# Bitsave SaveFi Protocol

[![Program ID](https://img.shields.io/badge/Program_ID-Fg6PaFpo...sLnS-blue.svg)](https://explorer.solana.com/address/Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS)

Bitsave is a native Solana program implementing **SaveFi** - a decentralized savings protocol. Users can create time-locked savings plans in supported stablecoins, earn commitment through lockups, and face penalties for early withdrawals. Fees fund treasury and buyback mechanisms.

## 🚀 Features

- **Factory Configuration**: Set fees, treasury/buyback wallets, up to 10 stablecoins.
- **User Profiles**: Track registration, savings count, total principal.
- **Savings Plans**: Custom name, amount, lock duration (seconds), penalty rate (1-5%).
  - Top-up anytime before maturity.
  - Early withdraw: Penalty split to treasury/buyback.
  - Mature withdraw: Full principal returned.
- **Secure**: PDAs for all state, ATA vaults, SPL token transfers, rent-exempt accounts.
- **Tested**: Full e2e tests including early/mature withdraws.

## 📋 Instructions

| Instruction | Accounts (key ones) | Description |
|-------------|---------------------|-------------|
| `InitializeFactory` | authority, treasury, buyback, factory PDA, system | Init factory with fees & stablecoins. |
| `RegisterUser` | user, factory, profile PDA, USDC ATAs, token/system | Pay registration fee (split), create profile. |
| `CreateSavingsPlan` | user, factory/profile/plan PDAs, ATAs, token/system | Lock principal + creation fee. |
| `TopUpSavings` | user, profile/plan PDAs, ATAs, token | Add to principal pre-unlock. |
| `WithdrawSavings` | user, factory/profile/plan PDAs, ATAs, token | Penalty if early, full if mature; close accounts. |

**Program ID**: `Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS`

## 🏗️ Architecture

```
Factory PDA ──┬──> UserProfile PDA ───┐
              │                       │
Treasury/Buyback ATAs     SavingsPlan PDA ──> Plan Vault ATA
              │                       │
              └──> Fees (reg/creation) │ Penalty (early withdraw)
                                      └── Principal (mature)
```

- **State Sizes**: Fixed-length borsh for rent-exemption.
- **PDAs**: Deterministic seeds (factory, user/{user}, savings/{user}/{index}).

## 🔧 Quickstart

### Prerequisites
- Rust & Solana CLI
- `solana-keygen new` (or use existing)

### Build & Test

**Linux/macOS:**
```bash
cargo test  # Run unit tests
cargo build-sbf
```

**Windows (cmd/PowerShell):**
```cmd
cargo test  # Run unit tests
set USERPROFILE=C:\Users\PC && cargo build-sbf
```
**Note:** `cargo build-sbf` requires USERPROFILE env var for Solana toolchain cache. Adjust path if your user folder differs. Install Solana CLI if missing: `sh -c "$(curl -sSfL https://release.solana.com/stable/install)"` (use WSL2 recommended).

### Deploy
```bash
# Build (see Build & Test above)

# Deploy (mainnet-beta example)
solana program deploy target/deploy/bitsave.so
```

### Client Integration
Use `solana-program` & `borsh` to construct instructions. See `tests/bitsave.rs` for examples.

## 📊 State Schemas

- **FactoryConfig**: authority, treasury/buyback, fees, total_users, stablecoins[10], bump.
- **UserProfile**: owner, registered_at, savings_count, total_principal, bump, initialized.
- **SavingsPlan**: owner, index, name[32], mint, principal, created/unlock_time, penalty, active, bump.

## 🧪 Tests

- `registration_create_topup_and_early_withdraw_work()`: Full flow + 2% early penalty.
- `mature_withdraw_returns_full_principal()`: Post-lockup full return.

All tests pass with USDC/USDT.

## 🔒 Security

- Signer & owner checks everywhere.
- PDA verification.
- Sufficient funds pre-transfers.
- Math overflow protection.
- Duplicate/empty stablecoins rejected.
- Invalid PDAs/ATAs rejected.

## 📝 License

MIT

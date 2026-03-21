use anchor_lang::prelude::*;

pub const NAME_MAX_LEN: usize = 64;
pub const MAX_SUPPORTED_STABLECOINS: usize = 16;
pub const SECONDS_PER_YEAR: i64 = 31_536_000;

pub const FACTORY_SEED: &[u8] = b"factory";
pub const USER_PROFILE_SEED: &[u8] = b"user";
pub const SAVINGS_SEED: &[u8] = b"savings";
pub const SAVINGS_VAULT_SEED: &[u8] = b"savings_vault";
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";
pub const INTEREST_VAULT_SEED: &[u8] = b"vault";
pub const INTEREST_VAULT_TOKEN_ACCOUNT_SEED: &[u8] = b"vault_token";

#[account]
pub struct FactoryConfig {
    pub authority: Pubkey,
    pub treasury_wallet: Pubkey,
    pub buyback_wallet: Pubkey,
    pub native_token_mint: Pubkey,
    pub registration_fee: u64,
    pub savings_creation_fee: u64,
    pub total_users: u64,
    pub interest_rate_basis_points: u64,
    pub supported_stablecoins: Vec<Pubkey>,
    pub interest_vault_bump: u8,
    pub bump: u8,
}

impl FactoryConfig {
    pub const SPACE: usize = 8
        + 32 * 4
        + 8 * 4
        + (4 + 32 * MAX_SUPPORTED_STABLECOINS)
        + 1
        + 1;

    pub const SEED: &'static [u8] = FACTORY_SEED;
}

#[account]
pub struct UserProfile {
    pub owner: Pubkey,
    pub registered_at: i64,
    pub savings_count: u64,
    pub total_principal: u64,
    pub bump: u8,
}

impl UserProfile {
    pub const SPACE: usize = 8 + 32 + 8 + 8 + 8 + 1;
    pub const SEED: &'static [u8] = USER_PROFILE_SEED;
}

#[account]
pub struct SavingsPlan {
    pub owner: Pubkey,
    pub plan_index: u64,
    pub name: String,
    pub stablecoin_mint: Pubkey,
    pub principal_amount: u64,
    pub created_at: i64,
    pub unlock_time: i64,
    pub penalty_rate: u8,
    pub interest_rate_basis_points: u64,
    pub interest_accrued: u64,
    pub last_claim_time: i64,
    pub is_active: bool,
    pub bump: u8,
}

impl SavingsPlan {
    pub const SPACE: usize = 8
        + 32
        + 8
        + 4
        + NAME_MAX_LEN
        + 32
        + 8
        + 8
        + 8
        + 1
        + 8
        + 8
        + 8
        + 1
        + 1;

    pub const SEED: &'static [u8] = SAVINGS_SEED;
    pub const TOKEN_VAULT_SEED: &'static [u8] = SAVINGS_VAULT_SEED;
}

#[account]
pub struct InterestVault {
    pub total_allocated: u64,
    pub total_claimed: u64,
    pub bump: u8,
    pub token_account_bump: u8,
}

impl InterestVault {
    pub const SPACE: usize = 8 + 8 + 8 + 1 + 1;
    pub const SEED: &'static [u8] = INTEREST_VAULT_SEED;
    pub const TOKEN_ACCOUNT_SEED: &'static [u8] = INTEREST_VAULT_TOKEN_ACCOUNT_SEED;
}

#[account]
pub struct StablecoinVault {
    pub mint: Pubkey,
    pub total_locked: u64,
    pub bump: u8,
}

impl StablecoinVault {
    pub const SPACE: usize = 8 + 32 + 8 + 1;
    pub const SEED: &'static [u8] = TOKEN_VAULT_SEED;
}

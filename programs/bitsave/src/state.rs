use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const MAX_SUPPORTED_STABLECOINS: usize = 10;
pub const MAX_PLAN_NAME_BYTES: usize = 32;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct FactoryConfig {
    pub authority: Pubkey,
    pub treasury_wallet: Pubkey,
    pub buyback_wallet: Pubkey,
    pub registration_fee: u64,
    pub savings_creation_fee: u64,
    pub total_users: u64,
    pub supported_stablecoins: Vec<Pubkey>,
    pub bump: u8,
}

impl FactoryConfig {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 8 + 4 + (32 * MAX_SUPPORTED_STABLECOINS) + 1;

    pub fn registration_mint(&self) -> Option<Pubkey> {
        self.supported_stablecoins.first().copied()
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct UserProfile {
    pub owner: Pubkey,
    pub registered_at: i64,
    pub savings_count: u64,
    pub total_principal: u64,
    pub bump: u8,
    pub is_initialized: bool,
}

impl UserProfile {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 1 + 1;
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct SavingsPlan {
    pub owner: Pubkey,
    pub plan_index: u64,
    pub name: String,
    pub stablecoin_mint: Pubkey,
    pub principal_amount: u64,
    pub created_at: i64,
    pub unlock_time: i64,
    pub penalty_rate: u8,
    pub is_active: bool,
    pub bump: u8,
}

impl SavingsPlan {
    pub const LEN: usize = 32 + 8 + 4 + MAX_PLAN_NAME_BYTES + 32 + 8 + 8 + 8 + 1 + 1 + 1;
}

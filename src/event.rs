use anchor_lang::prelude::*;

#[event]
pub struct SavingsCreated {
    pub user: Pubkey,
    pub plan_index: u64,
    pub name: String,
    pub amount: u64,
    pub unlock_time: i64,
    pub penalty_rate: u8,
}

#[event]
pub struct InterestClaimed {
    pub user: Pubkey,
    pub plan_index: u64,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct EarlyWithdrawal {
    pub user: Pubkey,
    pub plan_index: u64,
    pub principal: u64,
    pub penalty_amount: u64,
    pub returned_amount: u64,
}

#[event]
pub struct MatureWithdrawal {
    pub user: Pubkey,
    pub plan_index: u64,
    pub principal: u64,
    pub interest_claimed: u64,
}

#[event]
pub struct StablecoinAdded {
    pub mint: Pubkey,
}

#[event]
pub struct InterestVaultFunded {
    pub authority: Pubkey,
    pub amount: u64,
}

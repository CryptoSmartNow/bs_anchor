use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized = 6000,
    #[msg("Penalty rate must be between 1 and 5")]
    InvalidPenaltyRate = 6001,
    #[msg("Amount must be greater than zero")]
    InvalidAmount = 6002,
    #[msg("Stablecoin is not supported")]
    UnsupportedStablecoin = 6003,
    #[msg("Savings plan was not found")]
    PlanNotFound = 6004,
    #[msg("Savings plan is not active")]
    PlanNotActive = 6005,
    #[msg("Plan is locked and cannot be modified")]
    PlanLocked = 6006,
    #[msg("Insufficient vault funds")]
    InsufficientFunds = 6007,
    #[msg("Math overflow")]
    MathOverflow = 6008,
    #[msg("User already registered")]
    AlreadyRegistered = 6009,
    #[msg("User not registered")]
    NotRegistered = 6010,
    #[msg("Lock duration must be positive")]
    InvalidLockDuration = 6011,
}

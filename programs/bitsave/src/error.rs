use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum BitsaveError {
    #[error("Unauthorized access")]
    Unauthorized = 6000,
    #[error("Invalid penalty rate, must be 1-5")]
    InvalidPenaltyRate = 6001,
    #[error("Invalid amount, must be greater than 0")]
    InvalidAmount = 6002,
    #[error("Unsupported stablecoin")]
    UnsupportedStablecoin = 6003,
    #[error("Savings plan not found")]
    PlanNotFound = 6004,
    #[error("Savings plan not active")]
    PlanNotActive = 6005,
    #[error("Plan is locked and cannot be modified")]
    PlanLocked = 6006,
    #[error("Insufficient funds")]
    InsufficientFunds = 6007,
    #[error("Math overflow")]
    MathOverflow = 6008,
    #[error("User already registered")]
    AlreadyRegistered = 6009,
    #[error("User not registered")]
    NotRegistered = 6010,
    #[error("Invalid lock duration")]
    InvalidLockDuration = 6011,
    #[error("Name too long, max 32 bytes")]
    NameTooLong = 6012,
    #[error("Invalid fee amount")]
    InvalidFee = 6013,
    #[error("Treasury and buyback wallets must be different")]
    InvalidFeeRecipients = 6014,
    #[error("Too many supported stablecoins")]
    TooManySupportedStablecoins = 6015,
    #[error("Duplicate stablecoin mint in supported list")]
    DuplicateStablecoin = 6016,
    #[error("Token account mint does not match the expected mint")]
    InvalidMint = 6017,
    #[error("Plan vault does not belong to this savings plan")]
    InvalidPlanVault = 6018,
    #[error("Savings plan has already matured")]
    PlanMatured = 6019,
    #[error("Invalid PDA")]
    InvalidPda = 6020,
    #[error("Account not rent exempt")]
    NotRentExempt = 6021,
    #[error("Account already initialized")]
    AlreadyInitialized = 6022,
    #[error("Invalid account owner")]
    InvalidAccountOwner = 6023,
    #[error("Invalid account data")]
    InvalidAccountData = 6024,
}

impl From<BitsaveError> for ProgramError {
    fn from(value: BitsaveError) -> Self {
        ProgramError::Custom(value as u32)
    }
}


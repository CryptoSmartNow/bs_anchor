use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub enum BitsaveInstruction {
    InitializeFactory {
        registration_fee: u64,
        savings_creation_fee: u64,
        supported_stablecoins: Vec<Pubkey>,
    },
    RegisterUser,
    CreateSavingsPlan {
        name: String,
        amount: u64,
        lock_duration_seconds: i64,
        penalty_rate: u8,
    },
    TopUpSavings {
        plan_index: u64,
        additional_amount: u64,
    },
    WithdrawSavings {
        plan_index: u64,
    },
}

impl BitsaveInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(input).map_err(|_| ProgramError::InvalidInstructionData)
    }
}


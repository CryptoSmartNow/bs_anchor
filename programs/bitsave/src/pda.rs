use solana_program::pubkey::Pubkey;

pub fn factory_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"factory"], program_id)
}

pub fn user_profile_pda(program_id: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"user", owner.as_ref()], program_id)
}

pub fn savings_plan_pda(program_id: &Pubkey, owner: &Pubkey, plan_index: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"savings", owner.as_ref(), &plan_index.to_le_bytes()],
        program_id,
    )
}


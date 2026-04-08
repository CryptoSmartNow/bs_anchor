use std::collections::BTreeSet;

use borsh::{to_vec, BorshDeserialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{
    error::BitsaveError,
    instruction::BitsaveInstruction,
    pda,
    state::{FactoryConfig, SavingsPlan, UserProfile, MAX_PLAN_NAME_BYTES, MAX_SUPPORTED_STABLECOINS},
    token,
};

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        match BitsaveInstruction::unpack(instruction_data)? {
            BitsaveInstruction::InitializeFactory {
                registration_fee,
                savings_creation_fee,
                supported_stablecoins,
            } => Self::process_initialize_factory(
                program_id,
                accounts,
                registration_fee,
                savings_creation_fee,
                supported_stablecoins,
            ),
            BitsaveInstruction::RegisterUser => Self::process_register_user(program_id, accounts),
            BitsaveInstruction::CreateSavingsPlan {
                name,
                amount,
                lock_duration_seconds,
                penalty_rate,
            } => Self::process_create_savings(
                program_id,
                accounts,
                name,
                amount,
                lock_duration_seconds,
                penalty_rate,
            ),
            BitsaveInstruction::TopUpSavings {
                plan_index,
                additional_amount,
            } => Self::process_top_up_savings(program_id, accounts, plan_index, additional_amount),
            BitsaveInstruction::WithdrawSavings { plan_index } => {
                Self::process_withdraw_savings(program_id, accounts, plan_index)
            }
        }
    }

    fn process_initialize_factory(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        registration_fee: u64,
        savings_creation_fee: u64,
        supported_stablecoins: Vec<Pubkey>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let authority = next_account_info(account_info_iter)?;
        let treasury_wallet = next_account_info(account_info_iter)?;
        let buyback_wallet = next_account_info(account_info_iter)?;
        let factory = next_account_info(account_info_iter)?;
        let system_program_account = next_account_info(account_info_iter)?;

        assert_signer(authority)?;
        assert_system_program(system_program_account)?;

        if treasury_wallet.key == buyback_wallet.key {
            return Err(BitsaveError::InvalidFeeRecipients.into());
        }
        if registration_fee == 0 || savings_creation_fee == 0 {
            return Err(BitsaveError::InvalidFee.into());
        }
        if supported_stablecoins.is_empty() {
            return Err(BitsaveError::UnsupportedStablecoin.into());
        }
        if supported_stablecoins.len() > MAX_SUPPORTED_STABLECOINS {
            return Err(BitsaveError::TooManySupportedStablecoins.into());
        }

        let unique = supported_stablecoins.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != supported_stablecoins.len() {
            return Err(BitsaveError::DuplicateStablecoin.into());
        }

        let (expected_factory, bump) = pda::factory_pda(program_id);
        if expected_factory != *factory.key {
            return Err(BitsaveError::InvalidPda.into());
        }

        create_pda_account(
            authority,
            factory,
            system_program_account,
            program_id,
            &[b"factory", &[bump]],
            FactoryConfig::LEN,
        )?;

        let state = FactoryConfig {
            authority: *authority.key,
            treasury_wallet: *treasury_wallet.key,
            buyback_wallet: *buyback_wallet.key,
            registration_fee,
            savings_creation_fee,
            total_users: 0,
            supported_stablecoins,
            bump,
        };
        write_state(factory, &state)?;
        Ok(())
    }

    fn process_register_user(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let factory = next_account_info(account_info_iter)?;
        let user_profile = next_account_info(account_info_iter)?;
        let user_usdc = next_account_info(account_info_iter)?;
        let treasury_usdc = next_account_info(account_info_iter)?;
        let buyback_usdc = next_account_info(account_info_iter)?;
        let usdc_mint = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        let system_program_account = next_account_info(account_info_iter)?;

        assert_signer(user)?;
        assert_program_owner(factory, program_id)?;
        assert_system_program(system_program_account)?;

        let mut factory_state = read_state::<FactoryConfig>(factory)?;
        let registration_mint = factory_state
            .registration_mint()
            .ok_or(BitsaveError::UnsupportedStablecoin)?;
        if registration_mint != *usdc_mint.key {
            return Err(BitsaveError::InvalidMint.into());
        }

        let (expected_profile, bump) = pda::user_profile_pda(program_id, user.key);
        if expected_profile != *user_profile.key {
            return Err(BitsaveError::InvalidPda.into());
        }

        if user_profile.data_is_empty() || *user_profile.owner == system_program::id() {
            create_pda_account(
                user,
                user_profile,
                system_program_account,
                program_id,
                &[b"user", user.key.as_ref(), &[bump]],
                UserProfile::LEN,
            )?;
        } else {
            assert_program_owner(user_profile, program_id)?;
            let existing = read_state::<UserProfile>(user_profile)?;
            if existing.is_initialized {
                return Err(BitsaveError::AlreadyRegistered.into());
            }
        }

        let user_usdc_account = token::unpack_token_account(user_usdc)?;
        let treasury_account = token::unpack_token_account(treasury_usdc)?;
        let buyback_account = token::unpack_token_account(buyback_usdc)?;

        if user_usdc_account.owner != *user.key {
            return Err(BitsaveError::Unauthorized.into());
        }
        if user_usdc_account.mint != registration_mint
            || treasury_account.mint != registration_mint
            || buyback_account.mint != registration_mint
        {
            return Err(BitsaveError::InvalidMint.into());
        }
        if treasury_account.owner != factory_state.treasury_wallet
            || buyback_account.owner != factory_state.buyback_wallet
        {
            return Err(BitsaveError::Unauthorized.into());
        }
        if user_usdc_account.amount < factory_state.registration_fee {
            return Err(BitsaveError::InsufficientFunds.into());
        }

        let treasury_share = factory_state
            .registration_fee
            .checked_div(2)
            .ok_or(BitsaveError::MathOverflow)?;
        let buyback_share = factory_state
            .registration_fee
            .checked_sub(treasury_share)
            .ok_or(BitsaveError::MathOverflow)?;

        token::transfer(
            token_program,
            user_usdc,
            treasury_usdc,
            user,
            None,
            treasury_share,
        )?;
        token::transfer(
            token_program,
            user_usdc,
            buyback_usdc,
            user,
            None,
            buyback_share,
        )?;

        let profile = UserProfile {
            owner: *user.key,
            registered_at: Clock::get()?.unix_timestamp,
            savings_count: 0,
            total_principal: 0,
            bump,
            is_initialized: true,
        };
        write_state(user_profile, &profile)?;

        factory_state.total_users = factory_state
            .total_users
            .checked_add(1)
            .ok_or(BitsaveError::MathOverflow)?;
        write_state(factory, &factory_state)?;

        Ok(())
    }

    fn process_create_savings(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        amount: u64,
        lock_duration_seconds: i64,
        penalty_rate: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let factory = next_account_info(account_info_iter)?;
        let user_profile = next_account_info(account_info_iter)?;
        let savings_plan = next_account_info(account_info_iter)?;
        let plan_vault = next_account_info(account_info_iter)?;
        let user_stablecoin = next_account_info(account_info_iter)?;
        let treasury_stablecoin = next_account_info(account_info_iter)?;
        let buyback_stablecoin = next_account_info(account_info_iter)?;
        let stablecoin_mint = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        let system_program_account = next_account_info(account_info_iter)?;

        assert_signer(user)?;
        assert_program_owner(factory, program_id)?;
        assert_program_owner(user_profile, program_id)?;
        assert_system_program(system_program_account)?;

        if amount == 0 || name.is_empty() {
            return Err(BitsaveError::InvalidAmount.into());
        }
        if name.as_bytes().len() > MAX_PLAN_NAME_BYTES {
            return Err(BitsaveError::NameTooLong.into());
        }
        if !(1..=5).contains(&penalty_rate) {
            return Err(BitsaveError::InvalidPenaltyRate.into());
        }
        if lock_duration_seconds <= 0 {
            return Err(BitsaveError::InvalidLockDuration.into());
        }

        let factory_state = read_state::<FactoryConfig>(factory)?;
        let mut user_profile_state = read_state::<UserProfile>(user_profile)?;
        if !user_profile_state.is_initialized || user_profile_state.owner != *user.key {
            return Err(BitsaveError::NotRegistered.into());
        }
        if !factory_state
            .supported_stablecoins
            .contains(stablecoin_mint.key)
        {
            return Err(BitsaveError::UnsupportedStablecoin.into());
        }

        let plan_index = user_profile_state.savings_count;
        let (expected_plan, bump) = pda::savings_plan_pda(program_id, user.key, plan_index);
        if expected_plan != *savings_plan.key {
            return Err(BitsaveError::InvalidPda.into());
        }

        create_pda_account(
            user,
            savings_plan,
            system_program_account,
            program_id,
            &[b"savings", user.key.as_ref(), &plan_index.to_le_bytes(), &[bump]],
            SavingsPlan::LEN,
        )?;

        let expected_vault = get_associated_token_address(savings_plan.key, stablecoin_mint.key);
        if expected_vault != *plan_vault.key {
            return Err(BitsaveError::InvalidPlanVault.into());
        }

        let user_token_account = token::unpack_token_account(user_stablecoin)?;
        let treasury_account = token::unpack_token_account(treasury_stablecoin)?;
        let buyback_account = token::unpack_token_account(buyback_stablecoin)?;
        let vault_account = token::unpack_token_account(plan_vault)?;

        if user_token_account.owner != *user.key {
            return Err(BitsaveError::Unauthorized.into());
        }
        if user_token_account.mint != *stablecoin_mint.key
            || treasury_account.mint != *stablecoin_mint.key
            || buyback_account.mint != *stablecoin_mint.key
            || vault_account.mint != *stablecoin_mint.key
        {
            return Err(BitsaveError::InvalidMint.into());
        }
        if vault_account.owner != *savings_plan.key {
            return Err(BitsaveError::InvalidPlanVault.into());
        }
        if treasury_account.owner != factory_state.treasury_wallet
            || buyback_account.owner != factory_state.buyback_wallet
        {
            return Err(BitsaveError::Unauthorized.into());
        }

        let total_required = amount
            .checked_add(factory_state.savings_creation_fee)
            .ok_or(BitsaveError::MathOverflow)?;
        if user_token_account.amount < total_required {
            return Err(BitsaveError::InsufficientFunds.into());
        }

        let treasury_share = factory_state
            .savings_creation_fee
            .checked_div(2)
            .ok_or(BitsaveError::MathOverflow)?;
        let buyback_share = factory_state
            .savings_creation_fee
            .checked_sub(treasury_share)
            .ok_or(BitsaveError::MathOverflow)?;

        token::transfer(
            token_program,
            user_stablecoin,
            treasury_stablecoin,
            user,
            None,
            treasury_share,
        )?;
        token::transfer(
            token_program,
            user_stablecoin,
            buyback_stablecoin,
            user,
            None,
            buyback_share,
        )?;
        token::transfer(
            token_program,
            user_stablecoin,
            plan_vault,
            user,
            None,
            amount,
        )?;

        let now = Clock::get()?.unix_timestamp;
        let plan = SavingsPlan {
            owner: *user.key,
            plan_index,
            name,
            stablecoin_mint: *stablecoin_mint.key,
            principal_amount: amount,
            created_at: now,
            unlock_time: now
                .checked_add(lock_duration_seconds)
                .ok_or(BitsaveError::MathOverflow)?,
            penalty_rate,
            is_active: true,
            bump,
        };
        write_state(savings_plan, &plan)?;

        user_profile_state.savings_count = user_profile_state
            .savings_count
            .checked_add(1)
            .ok_or(BitsaveError::MathOverflow)?;
        user_profile_state.total_principal = user_profile_state
            .total_principal
            .checked_add(amount)
            .ok_or(BitsaveError::MathOverflow)?;
        write_state(user_profile, &user_profile_state)?;

        Ok(())
    }

    fn process_top_up_savings(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        plan_index: u64,
        additional_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let user_profile = next_account_info(account_info_iter)?;
        let savings_plan = next_account_info(account_info_iter)?;
        let plan_vault = next_account_info(account_info_iter)?;
        let user_stablecoin = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        assert_signer(user)?;
        assert_program_owner(user_profile, program_id)?;
        assert_program_owner(savings_plan, program_id)?;

        if additional_amount == 0 {
            return Err(BitsaveError::InvalidAmount.into());
        }

        let mut profile = read_state::<UserProfile>(user_profile)?;
        let mut plan = read_state::<SavingsPlan>(savings_plan)?;
        if !profile.is_initialized || profile.owner != *user.key {
            return Err(BitsaveError::NotRegistered.into());
        }
        if plan.owner != *user.key {
            return Err(BitsaveError::Unauthorized.into());
        }
        if !plan.is_active {
            return Err(BitsaveError::PlanNotActive.into());
        }
        if Clock::get()?.unix_timestamp >= plan.unlock_time {
            return Err(BitsaveError::PlanMatured.into());
        }

        let (expected_plan, _) = pda::savings_plan_pda(program_id, user.key, plan_index);
        if expected_plan != *savings_plan.key || plan.plan_index != plan_index {
            return Err(BitsaveError::InvalidPda.into());
        }

        let expected_vault = get_associated_token_address(savings_plan.key, &plan.stablecoin_mint);
        if expected_vault != *plan_vault.key {
            return Err(BitsaveError::InvalidPlanVault.into());
        }

        let user_token_account = token::unpack_token_account(user_stablecoin)?;
        let vault_account = token::unpack_token_account(plan_vault)?;
        if user_token_account.owner != *user.key {
            return Err(BitsaveError::Unauthorized.into());
        }
        if user_token_account.mint != plan.stablecoin_mint || vault_account.mint != plan.stablecoin_mint
        {
            return Err(BitsaveError::InvalidMint.into());
        }
        if vault_account.owner != *savings_plan.key {
            return Err(BitsaveError::InvalidPlanVault.into());
        }

        token::transfer(
            token_program,
            user_stablecoin,
            plan_vault,
            user,
            None,
            additional_amount,
        )?;

        plan.principal_amount = plan
            .principal_amount
            .checked_add(additional_amount)
            .ok_or(BitsaveError::MathOverflow)?;
        profile.total_principal = profile
            .total_principal
            .checked_add(additional_amount)
            .ok_or(BitsaveError::MathOverflow)?;

        write_state(savings_plan, &plan)?;
        write_state(user_profile, &profile)?;
        Ok(())
    }

    fn process_withdraw_savings(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        plan_index: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let user_profile = next_account_info(account_info_iter)?;
        let factory = next_account_info(account_info_iter)?;
        let savings_plan = next_account_info(account_info_iter)?;
        let plan_vault = next_account_info(account_info_iter)?;
        let user_stablecoin = next_account_info(account_info_iter)?;
        let treasury_stablecoin = next_account_info(account_info_iter)?;
        let buyback_stablecoin = next_account_info(account_info_iter)?;
        let stablecoin_mint = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        assert_signer(user)?;
        assert_program_owner(user_profile, program_id)?;
        assert_program_owner(factory, program_id)?;
        assert_program_owner(savings_plan, program_id)?;

        let mut profile = read_state::<UserProfile>(user_profile)?;
        let factory_state = read_state::<FactoryConfig>(factory)?;
        let plan = read_state::<SavingsPlan>(savings_plan)?;

        if !profile.is_initialized || profile.owner != *user.key {
            return Err(BitsaveError::NotRegistered.into());
        }
        if plan.owner != *user.key {
            return Err(BitsaveError::Unauthorized.into());
        }
        if !plan.is_active {
            return Err(BitsaveError::PlanNotActive.into());
        }

        let (expected_plan, _) = pda::savings_plan_pda(program_id, user.key, plan_index);
        if expected_plan != *savings_plan.key || plan.plan_index != plan_index {
            return Err(BitsaveError::InvalidPda.into());
        }

        let expected_vault = get_associated_token_address(savings_plan.key, &plan.stablecoin_mint);
        if expected_vault != *plan_vault.key {
            return Err(BitsaveError::InvalidPlanVault.into());
        }
        if plan.stablecoin_mint != *stablecoin_mint.key {
            return Err(BitsaveError::InvalidMint.into());
        }

        let user_token_account = token::unpack_token_account(user_stablecoin)?;
        let treasury_account = token::unpack_token_account(treasury_stablecoin)?;
        let buyback_account = token::unpack_token_account(buyback_stablecoin)?;
        let vault_account = token::unpack_token_account(plan_vault)?;

        if user_token_account.owner != *user.key {
            return Err(BitsaveError::Unauthorized.into());
        }
        if user_token_account.mint != plan.stablecoin_mint
            || treasury_account.mint != plan.stablecoin_mint
            || buyback_account.mint != plan.stablecoin_mint
            || vault_account.mint != plan.stablecoin_mint
        {
            return Err(BitsaveError::InvalidMint.into());
        }
        if treasury_account.owner != factory_state.treasury_wallet
            || buyback_account.owner != factory_state.buyback_wallet
        {
            return Err(BitsaveError::Unauthorized.into());
        }
        if vault_account.owner != *savings_plan.key {
            return Err(BitsaveError::InvalidPlanVault.into());
        }

        let principal = plan.principal_amount;
        if principal == 0 {
            return Err(BitsaveError::InvalidAmount.into());
        }

        let now = Clock::get()?.unix_timestamp;
        let signer_seeds: &[&[u8]] = &[
            b"savings",
            user.key.as_ref(),
            &plan_index.to_le_bytes(),
            &[plan.bump],
        ];

        if now < plan.unlock_time {
            let penalty = principal
                .checked_mul(plan.penalty_rate as u64)
                .ok_or(BitsaveError::MathOverflow)?
                .checked_div(100)
                .ok_or(BitsaveError::MathOverflow)?;
            let treasury_share = penalty
                .checked_div(2)
                .ok_or(BitsaveError::MathOverflow)?;
            let buyback_share = penalty
                .checked_sub(treasury_share)
                .ok_or(BitsaveError::MathOverflow)?;
            let user_amount = principal
                .checked_sub(penalty)
                .ok_or(BitsaveError::MathOverflow)?;

            token::transfer(
                token_program,
                plan_vault,
                treasury_stablecoin,
                savings_plan,
                Some(signer_seeds),
                treasury_share,
            )?;
            token::transfer(
                token_program,
                plan_vault,
                buyback_stablecoin,
                savings_plan,
                Some(signer_seeds),
                buyback_share,
            )?;
            token::transfer(
                token_program,
                plan_vault,
                user_stablecoin,
                savings_plan,
                Some(signer_seeds),
                user_amount,
            )?;
        } else {
            token::transfer(
                token_program,
                plan_vault,
                user_stablecoin,
                savings_plan,
                Some(signer_seeds),
                principal,
            )?;
        }

        token::close_account(token_program, plan_vault, user, savings_plan, signer_seeds)?;

        profile.total_principal = profile
            .total_principal
            .checked_sub(principal)
            .ok_or(BitsaveError::MathOverflow)?;
        write_state(user_profile, &profile)?;

        close_program_account(savings_plan, user)?;
        msg!("Savings plan {} withdrawn", plan_index);
        Ok(())
    }
}

fn assert_signer(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

fn assert_system_program(account: &AccountInfo) -> ProgramResult {
    if *account.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

fn assert_program_owner(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        return Err(BitsaveError::InvalidAccountOwner.into());
    }
    Ok(())
}

// THIS IS THE ONLY CHANGED FUNCTION
fn read_state<T: BorshDeserialize>(account: &AccountInfo) -> Result<T, ProgramError> {
    let data = account.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    T::deserialize(&mut slice).map_err(|_| BitsaveError::InvalidAccountData.into())
}

fn write_state<T: borsh::BorshSerialize>(account: &AccountInfo, state: &T) -> ProgramResult {
    let bytes = to_vec(state).map_err(|_| BitsaveError::InvalidAccountData)?;
    if bytes.len() > account.data_len() {
        return Err(BitsaveError::InvalidAccountData.into());
    }
    let mut data = account.try_borrow_mut_data()?;
    data.fill(0);
    data[..bytes.len()].copy_from_slice(&bytes);
    Ok(())
}

fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    seeds: &[&[u8]],
    space: usize,
) -> ProgramResult {
    if !target.data_is_empty() || *target.owner == *program_id {
        return Err(BitsaveError::AlreadyInitialized.into());
    }
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);
    let ix = system_instruction::create_account(
        payer.key,
        target.key,
        lamports,
        space as u64,
        program_id,
    );
    invoke_signed(
        &ix,
        &[payer.clone(), target.clone(), system_program_account.clone()],
        &[seeds],
    )
}

fn close_program_account(source: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
    let lamports = source.lamports();
    **source.try_borrow_mut_lamports()? -= lamports;
    **destination.try_borrow_mut_lamports()? += lamports;
    source.try_borrow_mut_data()?.fill(0);
    Ok(())
}
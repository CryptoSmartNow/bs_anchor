use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
};
use spl_token::{instruction as token_instruction, state::Account as TokenAccount, ID as TOKEN_PROGRAM_ID};

use crate::error::BitsaveError;

pub fn unpack_token_account(account: &AccountInfo) -> Result<TokenAccount, ProgramError> {
    if *account.owner != TOKEN_PROGRAM_ID {
        return Err(BitsaveError::InvalidAccountOwner.into());
    }
    TokenAccount::unpack(&account.try_borrow_data()?).map_err(|_| BitsaveError::InvalidAccountData.into())
}

pub fn transfer<'a>(
    token_program: &AccountInfo<'a>,
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    signer_seeds: Option<&[&[u8]]>,
    amount: u64,
) -> Result<(), ProgramError> {
    let ix = token_instruction::transfer(
        token_program.key,
        from.key,
        to.key,
        authority.key,
        &[],
        amount,
    )?;
    match signer_seeds {
        Some(seeds) => invoke_signed(
            &ix,
            &[from.clone(), to.clone(), authority.clone(), token_program.clone()],
            &[seeds],
        ),
        None => invoke(
            &ix,
            &[from.clone(), to.clone(), authority.clone(), token_program.clone()],
        ),
    }
}

pub fn close_account<'a>(
    token_program: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let ix = token_instruction::close_account(
        token_program.key,
        account.key,
        destination.key,
        authority.key,
        &[],
    )?;
    invoke_signed(
        &ix,
        &[
            account.clone(),
            destination.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        &[signer_seeds],
    )
}

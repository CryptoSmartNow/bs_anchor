use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::error::ErrorCode;
use crate::state::{FactoryConfig, StablecoinVault};

pub fn assert_supported_stablecoin(factory: &FactoryConfig, mint: &Pubkey) -> Result<()> {
    if factory
        .supported_stablecoins
        .iter()
        .any(|entry| entry == mint)
    {
        Ok(())
    } else {
        Err(ErrorCode::UnsupportedStablecoin.into())
    }
}

pub fn split_fee<'info>(
    amount: u64,
    payer: &AccountInfo<'info>,
    payer_token_account: &Account<'info, TokenAccount>,
    treasury_token_account: &Account<'info, TokenAccount>,
    buyback_token_account: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidAmount);

    let half = amount
        .checked_div(2)
        .ok_or(ErrorCode::MathOverflow)?;
    let remainder = amount
        .checked_sub(half)
        .ok_or(ErrorCode::MathOverflow)?;

    transfer_tokens(payer_token_account, treasury_token_account, payer, token_program, half)?;
    transfer_tokens(payer_token_account, buyback_token_account, payer, token_program, remainder)?;
    Ok(())
}

pub fn update_locked_amount(
    vault: &mut StablecoinVault,
    amount: u64,
    increase: bool,
) -> Result<()> {
    if increase {
        vault.total_locked = vault
            .total_locked
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;
    } else {
        vault.total_locked = vault
            .total_locked
            .checked_sub(amount)
            .ok_or(ErrorCode::MathOverflow)?;
    }
    Ok(())
}

fn transfer_tokens<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let ctx = CpiContext::new(
        token_program.to_account_info(),
        Transfer {
            from: from.to_account_info(),
            to: to.to_account_info(),
            authority: authority.clone(),
        },
    );
    token::transfer(ctx, amount)?;
    Ok(())
}

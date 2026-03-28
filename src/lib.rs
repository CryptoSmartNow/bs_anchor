use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use std::convert::TryFrom;

pub mod error;
pub mod event;
pub mod state;
pub mod util;

use crate::error::ErrorCode;
use crate::event::*;
use crate::state::*;
use crate::util::*;

declare_id!("6bS35BvXGd6vu8PUaruDcoPPb37aK6QT1RH3mq1TZ8Fg");

#[program]
pub mod bs_anchor {
    use super::*;

    pub fn initialize_factory(
        ctx: Context<InitializeFactory>,
        registration_fee: u64,
        savings_creation_fee: u64,
        interest_rate_basis_points: u64,
    ) -> Result<()> {
        require!(savings_creation_fee > 0, ErrorCode::InvalidAmount);

        let factory = &mut ctx.accounts.factory;
        factory.authority = ctx.accounts.authority.key();
        factory.treasury_wallet = ctx.accounts.treasury_wallet.key();
        factory.buyback_wallet = ctx.accounts.buyback_wallet.key();
        factory.native_token_mint = ctx.accounts.native_token_mint.key();
        factory.registration_fee = registration_fee;
        factory.savings_creation_fee = savings_creation_fee;
        factory.total_users = 0;
        factory.interest_rate_basis_points = interest_rate_basis_points;
        factory.supported_stablecoins = vec![
            ctx.accounts.usdc_mint.key(),
            ctx.accounts.usdt_mint.key(),
        ];
        factory.bump = ctx.bumps.factory;
        factory.interest_vault_bump = ctx.bumps.interest_vault;

        let interest_vault = &mut ctx.accounts.interest_vault;
        interest_vault.total_allocated = 0;
        interest_vault.total_claimed = 0;
        interest_vault.bump = ctx.bumps.interest_vault;
        interest_vault.token_account_bump = ctx.bumps.interest_vault_token_account;

        ctx.accounts.usdc_vault.mint = ctx.accounts.usdc_mint.key();
        ctx.accounts.usdc_vault.total_locked = 0;
        ctx.accounts.usdc_vault.bump = ctx.bumps.usdc_vault;

        ctx.accounts.usdt_vault.mint = ctx.accounts.usdt_mint.key();
        ctx.accounts.usdt_vault.total_locked = 0;
        ctx.accounts.usdt_vault.bump = ctx.bumps.usdt_vault;

        Ok(())
    }

    pub fn register_user(ctx: Context<RegisterUser>) -> Result<()> {
        let profile = &mut ctx.accounts.user_profile;
        if profile.registered_at != 0 {
            return err!(ErrorCode::AlreadyRegistered);
        }

        assert_supported_stablecoin(&ctx.accounts.factory, &ctx.accounts.stablecoin_mint.key())?;

        if ctx.accounts.factory.registration_fee > 0 {
            split_fee(
                ctx.accounts.factory.registration_fee,
                &ctx.accounts.user.to_account_info(),
                &ctx.accounts.user_stablecoin_ata,
                &ctx.accounts.treasury_token_account,
                &ctx.accounts.buyback_token_account,
                &ctx.accounts.token_program,
            )?;
        }

        profile.owner = ctx.accounts.user.key();
        profile.registered_at = ctx.accounts.clock.unix_timestamp;
        profile.savings_count = 0;
        profile.total_principal = 0;
        profile.bump = ctx.bumps.user_profile;

        ctx.accounts.factory.total_users = ctx
            .accounts
            .factory
            .total_users
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(())
    }

    pub fn create_savings_plan(
        ctx: Context<CreateSavingsPlan>,
        name: String,
        amount: u64,
        lock_duration_seconds: i64,
        penalty_rate: u8,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.user_profile;
        require!(profile.registered_at != 0, ErrorCode::NotRegistered);
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(lock_duration_seconds > 0, ErrorCode::InvalidLockDuration);
        require!((1..=5).contains(&penalty_rate), ErrorCode::InvalidPenaltyRate);

        assert_supported_stablecoin(&ctx.accounts.factory, &ctx.accounts.stablecoin_mint.key())?;

        split_fee(
            ctx.accounts.factory.savings_creation_fee,
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.user_stablecoin_ata,
            &ctx.accounts.treasury_token_account,
            &ctx.accounts.buyback_token_account,
            &ctx.accounts.token_program,
        )?;

        let now = ctx.accounts.clock.unix_timestamp;

        let safe_name = if name.len() > NAME_MAX_LEN {
            name.chars().take(NAME_MAX_LEN).collect()
        } else {
            name
        };

        let plan = &mut ctx.accounts.savings_plan;
        plan.owner = ctx.accounts.user.key();
        plan.plan_index = profile.savings_count;
        plan.name = safe_name;
        plan.stablecoin_mint = ctx.accounts.stablecoin_mint.key();
        plan.principal_amount = amount;
        plan.created_at = now;
        plan.unlock_time = now
            .checked_add(lock_duration_seconds)
            .ok_or(ErrorCode::MathOverflow)?;
        plan.penalty_rate = penalty_rate;
        plan.interest_rate_basis_points = ctx.accounts.factory.interest_rate_basis_points;
        plan.interest_accrued = 0;
        plan.last_claim_time = now;
        plan.is_active = true;
        plan.bump = ctx.bumps.savings_plan;

        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_stablecoin_ata.to_account_info(),
                to: ctx.accounts.savings_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        profile.savings_count = profile
            .savings_count
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
        profile.total_principal = profile
            .total_principal
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;

        update_locked_amount(&mut ctx.accounts.stablecoin_vault, amount, true)?;

        emit!(SavingsCreated {
            user: ctx.accounts.user.key(),
            plan_index: plan.plan_index,
            name: plan.name.clone(),
            amount,
            unlock_time: plan.unlock_time,
            penalty_rate,
        });

        Ok(())
    }

    pub fn top_up_savings(
        ctx: Context<TopUpSavings>,
        _plan_index: u64,
        additional_amount: u64,
    ) -> Result<()> {
        require!(additional_amount > 0, ErrorCode::InvalidAmount);
        require!(
            ctx.accounts.user_profile.registered_at != 0,
            ErrorCode::NotRegistered
        );

        let plan = &mut ctx.accounts.savings_plan;
        require!(plan.is_active, ErrorCode::PlanNotActive);

        let now = ctx.accounts.clock.unix_timestamp;
        require!(now < plan.unlock_time, ErrorCode::PlanLocked);

        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_stablecoin_ata.to_account_info(),
                to: ctx.accounts.plan_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, additional_amount)?;

        plan.principal_amount = plan
            .principal_amount
            .checked_add(additional_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        ctx.accounts.user_profile.total_principal = ctx
            .accounts
            .user_profile
            .total_principal
            .checked_add(additional_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        update_locked_amount(&mut ctx.accounts.stablecoin_vault, additional_amount, true)?;

        Ok(())
    }

    pub fn claim_interest(ctx: Context<ClaimInterest>, plan_index: u64) -> Result<()> {
        require!(
            ctx.accounts.user_profile.registered_at != 0,
            ErrorCode::NotRegistered
        );
        require!(ctx.accounts.savings_plan.is_active, ErrorCode::PlanNotActive);

        let now = ctx.accounts.clock.unix_timestamp;
        let interest_amount = settle_interest(
            &mut ctx.accounts.savings_plan,
            &mut ctx.accounts.interest_vault,
            &ctx.accounts.interest_vault_token_account,
            &ctx.accounts.user_reward_ata,
            &ctx.accounts.token_program,
            &ctx.accounts.factory.key(),
            now,
        )?;

        if interest_amount > 0 {
            emit!(InterestClaimed {
                user: ctx.accounts.user.key(),
                plan_index,
                amount: interest_amount,
                timestamp: now,
            });
        }

        Ok(())
    }

    pub fn withdraw_savings(ctx: Context<WithdrawSavings>, plan_index: u64) -> Result<()> {
        let now = ctx.accounts.clock.unix_timestamp;
        let plan = &mut ctx.accounts.savings_plan;

        require!(plan.is_active, ErrorCode::PlanNotActive);
        require!(
            ctx.accounts.user_profile.registered_at != 0,
            ErrorCode::NotRegistered
        );

        let principal = plan.principal_amount;
        require!(principal > 0, ErrorCode::InvalidAmount);

        if now >= plan.unlock_time {
            let interest_amount = settle_interest(
                plan,
                &mut ctx.accounts.interest_vault,
                &ctx.accounts.interest_vault_token_account,
                &ctx.accounts.user_reward_ata,
                &ctx.accounts.token_program,
                &ctx.accounts.factory.key(),
                now,
            )?;

            transfer_from_plan_vault(
                principal,
                plan,
                &ctx.accounts.plan_vault,
                &ctx.accounts.user_stablecoin_ata,
                &ctx.accounts.token_program,
            )?;

            emit!(MatureWithdrawal {
                user: ctx.accounts.user.key(),
                plan_index,
                principal,
                interest_claimed: interest_amount,
            });
        } else {
            let penalty_amount = principal
                .checked_mul(plan.penalty_rate as u64)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(100)
                .ok_or(ErrorCode::MathOverflow)?;
            let returned_amount = principal
                .checked_sub(penalty_amount)
                .ok_or(ErrorCode::MathOverflow)?;

            transfer_from_plan_vault(
                returned_amount,
                plan,
                &ctx.accounts.plan_vault,
                &ctx.accounts.user_stablecoin_ata,
                &ctx.accounts.token_program,
            )?;

            let half_penalty = penalty_amount
                .checked_div(2)
                .ok_or(ErrorCode::MathOverflow)?;
            let remainder_penalty = penalty_amount
                .checked_sub(half_penalty)
                .ok_or(ErrorCode::MathOverflow)?;

            transfer_from_plan_vault(
                half_penalty,
                plan,
                &ctx.accounts.plan_vault,
                &ctx.accounts.treasury_token_account,
                &ctx.accounts.token_program,
            )?;
            transfer_from_plan_vault(
                remainder_penalty,
                plan,
                &ctx.accounts.plan_vault,
                &ctx.accounts.buyback_token_account,
                &ctx.accounts.token_program,
            )?;

            emit!(EarlyWithdrawal {
                user: ctx.accounts.user.key(),
                plan_index,
                principal,
                penalty_amount,
                returned_amount,
            });
        }

        plan.is_active = false;
        plan.principal_amount = 0;

        ctx.accounts.user_profile.total_principal = ctx
            .accounts
            .user_profile
            .total_principal
            .checked_sub(principal)
            .ok_or(ErrorCode::MathOverflow)?;

        update_locked_amount(&mut ctx.accounts.stablecoin_vault, principal, false)?;

        Ok(())
    }

    pub fn update_interest_rate(
        ctx: Context<UpdateInterestRate>,
        new_rate_basis_points: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.factory.authority,
            ErrorCode::Unauthorized
        );
        ctx.accounts.factory.interest_rate_basis_points = new_rate_basis_points;
        Ok(())
    }

    pub fn add_supported_stablecoin(ctx: Context<AddStablecoin>) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.factory.authority,
            ErrorCode::Unauthorized
        );
        require!(
            ctx.accounts.factory.supported_stablecoins.len() < MAX_SUPPORTED_STABLECOINS,
            ErrorCode::MathOverflow
        );

        let mint = ctx.accounts.new_stablecoin_mint.key();
        if ctx
            .accounts
            .factory
            .supported_stablecoins
            .iter()
            .any(|entry| entry == &mint)
        {
            return Err(ErrorCode::UnsupportedStablecoin.into());
        }

        ctx.accounts.stablecoin_vault.mint = mint;
        ctx.accounts.stablecoin_vault.total_locked = 0;
        ctx.accounts.stablecoin_vault.bump = ctx.bumps.stablecoin_vault;

        ctx.accounts.factory.supported_stablecoins.push(mint);

        emit!(StablecoinAdded { mint });
        Ok(())
    }

    pub fn fund_interest_vault(ctx: Context<FundInterestVault>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(
            ctx.accounts.authority.key() == ctx.accounts.factory.authority,
            ErrorCode::Unauthorized
        );

        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.funder_native_ata.to_account_info(),
                to: ctx.accounts.interest_vault_token_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        ctx.accounts.interest_vault.total_allocated = ctx
            .accounts
            .interest_vault
            .total_allocated
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;

        emit!(InterestVaultFunded {
            authority: ctx.accounts.authority.key(),
            amount,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(
    registration_fee: u64,
    savings_creation_fee: u64,
    interest_rate_basis_points: u64
)]
pub struct InitializeFactory<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [FactoryConfig::SEED],
        bump,
        space = FactoryConfig::SPACE
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        init,
        payer = authority,
        seeds = [InterestVault::SEED, factory.key().as_ref()],
        bump,
        space = InterestVault::SPACE
    )]
    pub interest_vault: Account<'info, InterestVault>,
    #[account(
        init,
        payer = authority,
        seeds = [InterestVault::TOKEN_ACCOUNT_SEED, factory.key().as_ref()],
        bump,
        token::mint = native_token_mint,
        token::authority = interest_vault
    )]
    pub interest_vault_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        seeds = [StablecoinVault::SEED, usdc_mint.key().as_ref()],
        bump,
        space = StablecoinVault::SPACE
    )]
    pub usdc_vault: Account<'info, StablecoinVault>,
    #[account(
        init,
        payer = authority,
        seeds = [StablecoinVault::SEED, usdt_mint.key().as_ref()],
        bump,
        space = StablecoinVault::SPACE
    )]
    pub usdt_vault: Account<'info, StablecoinVault>,

    pub native_token_mint: Account<'info, Mint>,
    pub usdc_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,

    /// CHECK: stored pubkey
    pub treasury_wallet: UncheckedAccount<'info>,
    /// CHECK: stored pubkey
    pub buyback_wallet: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct RegisterUser<'info> {
    #[account(
        mut,
        seeds = [FactoryConfig::SEED],
        bump = factory.bump
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        init_if_needed,
        payer = user,
        seeds = [UserProfile::SEED, user.key().as_ref()],
        bump,
        space = UserProfile::SPACE
    )]
    pub user_profile: Account<'info, UserProfile>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub stablecoin_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = user_stablecoin_ata.owner == user.key(),
        constraint = user_stablecoin_ata.mint == stablecoin_mint.key()
    )]
    pub user_stablecoin_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = treasury_token_account.owner == factory.treasury_wallet,
        constraint = treasury_token_account.mint == stablecoin_mint.key()
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyback_token_account.owner == factory.buyback_wallet,
        constraint = buyback_token_account.mint == stablecoin_mint.key()
    )]
    pub buyback_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct CreateSavingsPlan<'info> {
    #[account(
        mut,
        seeds = [FactoryConfig::SEED],
        bump = factory.bump
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        mut,
        seeds = [UserProfile::SEED, user.key().as_ref()],
        bump = user_profile.bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        seeds = [
            SavingsPlan::SEED,
            user.key().as_ref(),
            user_profile.savings_count.to_le_bytes().as_ref()
        ],
        bump,
        space = SavingsPlan::SPACE
    )]
    pub savings_plan: Account<'info, SavingsPlan>,
    #[account(
        init,
        payer = user,
        seeds = [SavingsPlan::TOKEN_VAULT_SEED, savings_plan.key().as_ref()],
        bump,
        token::mint = stablecoin_mint,
        token::authority = savings_plan
    )]
    pub savings_vault: Account<'info, TokenAccount>,

    pub stablecoin_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = user_stablecoin_ata.owner == user.key(),
        constraint = user_stablecoin_ata.mint == stablecoin_mint.key()
    )]
    pub user_stablecoin_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = treasury_token_account.owner == factory.treasury_wallet,
        constraint = treasury_token_account.mint == stablecoin_mint.key()
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyback_token_account.owner == factory.buyback_wallet,
        constraint = buyback_token_account.mint == stablecoin_mint.key()
    )]
    pub buyback_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [StablecoinVault::SEED, stablecoin_mint.key().as_ref()],
        bump = stablecoin_vault.bump
    )]
    pub stablecoin_vault: Account<'info, StablecoinVault>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(plan_index: u64)]
pub struct TopUpSavings<'info> {
    #[account(
        mut,
        seeds = [FactoryConfig::SEED],
        bump = factory.bump
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        mut,
        seeds = [UserProfile::SEED, user.key().as_ref()],
        bump = user_profile.bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [
            SavingsPlan::SEED,
            user.key().as_ref(),
            plan_index.to_le_bytes().as_ref()
        ],
        bump = savings_plan.bump
    )]
    pub savings_plan: Account<'info, SavingsPlan>,
    #[account(
        mut,
        constraint = plan_vault.mint == savings_plan.stablecoin_mint,
        constraint = plan_vault.owner == savings_plan.key()
    )]
    pub plan_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_stablecoin_ata.owner == user.key(),
        constraint = user_stablecoin_ata.mint == savings_plan.stablecoin_mint
    )]
    pub user_stablecoin_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [StablecoinVault::SEED, savings_plan.stablecoin_mint.as_ref()],
        bump = stablecoin_vault.bump
    )]
    pub stablecoin_vault: Account<'info, StablecoinVault>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(plan_index: u64)]
pub struct ClaimInterest<'info> {
    #[account(
        mut,
        seeds = [FactoryConfig::SEED],
        bump = factory.bump
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        mut,
        seeds = [UserProfile::SEED, user.key().as_ref()],
        bump = user_profile.bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [
            SavingsPlan::SEED,
            user.key().as_ref(),
            plan_index.to_le_bytes().as_ref()
        ],
        bump = savings_plan.bump
    )]
    pub savings_plan: Account<'info, SavingsPlan>,
    #[account(
        mut,
        constraint = user_reward_ata.owner == user.key(),
        constraint = user_reward_ata.mint == factory.native_token_mint
    )]
    pub user_reward_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [InterestVault::SEED, factory.key().as_ref()],
        bump = factory.interest_vault_bump
    )]
    pub interest_vault: Account<'info, InterestVault>,
    #[account(
        mut,
        seeds = [InterestVault::TOKEN_ACCOUNT_SEED, factory.key().as_ref()],
        bump = interest_vault.token_account_bump,
        constraint = interest_vault_token_account.owner == interest_vault.key(),
        constraint = interest_vault_token_account.mint == factory.native_token_mint
    )]
    pub interest_vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(plan_index: u64)]
pub struct WithdrawSavings<'info> {
    #[account(
        mut,
        seeds = [FactoryConfig::SEED],
        bump = factory.bump
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        mut,
        seeds = [UserProfile::SEED, user.key().as_ref()],
        bump = user_profile.bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [
            SavingsPlan::SEED,
            user.key().as_ref(),
            plan_index.to_le_bytes().as_ref()
        ],
        bump = savings_plan.bump,
        close = user
    )]
    pub savings_plan: Account<'info, SavingsPlan>,
    #[account(
        mut,
        constraint = plan_vault.mint == savings_plan.stablecoin_mint,
        constraint = plan_vault.owner == savings_plan.key()
    )]
    pub plan_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_stablecoin_ata.owner == user.key(),
        constraint = user_stablecoin_ata.mint == savings_plan.stablecoin_mint
    )]
    pub user_stablecoin_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = treasury_token_account.owner == factory.treasury_wallet,
        constraint = treasury_token_account.mint == savings_plan.stablecoin_mint
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyback_token_account.owner == factory.buyback_wallet,
        constraint = buyback_token_account.mint == savings_plan.stablecoin_mint
    )]
    pub buyback_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [StablecoinVault::SEED, savings_plan.stablecoin_mint.as_ref()],
        bump = stablecoin_vault.bump
    )]
    pub stablecoin_vault: Account<'info, StablecoinVault>,
    #[account(
        mut,
        seeds = [InterestVault::SEED, factory.key().as_ref()],
        bump = factory.interest_vault_bump
    )]
    pub interest_vault: Account<'info, InterestVault>,
    #[account(
        mut,
        seeds = [InterestVault::TOKEN_ACCOUNT_SEED, factory.key().as_ref()],
        bump = interest_vault.token_account_bump,
        constraint = interest_vault_token_account.owner == interest_vault.key(),
        constraint = interest_vault_token_account.mint == factory.native_token_mint
    )]
    pub interest_vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_reward_ata.owner == user.key(),
        constraint = user_reward_ata.mint == factory.native_token_mint
    )]
    pub user_reward_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct UpdateInterestRate<'info> {
    #[account(mut)]
    pub factory: Account<'info, FactoryConfig>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddStablecoin<'info> {
    #[account(mut)]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        init,
        payer = authority,
        seeds = [StablecoinVault::SEED, new_stablecoin_mint.key().as_ref()],
        bump,
        space = StablecoinVault::SPACE
    )]
    pub stablecoin_vault: Account<'info, StablecoinVault>,
    pub new_stablecoin_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct FundInterestVault<'info> {
    #[account(
        mut,
        seeds = [FactoryConfig::SEED],
        bump = factory.bump
    )]
    pub factory: Account<'info, FactoryConfig>,
    #[account(
        mut,
        seeds = [InterestVault::SEED, factory.key().as_ref()],
        bump = factory.interest_vault_bump
    )]
    pub interest_vault: Account<'info, InterestVault>,
    #[account(
        mut,
        seeds = [InterestVault::TOKEN_ACCOUNT_SEED, factory.key().as_ref()],
        bump = interest_vault.token_account_bump,
        constraint = interest_vault_token_account.owner == interest_vault.key(),
        constraint = interest_vault_token_account.mint == factory.native_token_mint
    )]
    pub interest_vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = funder_native_ata.owner == authority.key(),
        constraint = funder_native_ata.mint == factory.native_token_mint
    )]
    pub funder_native_ata: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

fn settle_interest<'info>(
    plan: &mut Account<'info, SavingsPlan>,
    interest_vault: &mut Account<'info, InterestVault>,
    interest_vault_token_account: &Account<'info, TokenAccount>,
    recipient: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    factory_key: &Pubkey,
    now: i64,
) -> Result<u64> {
    let interest = calculate_interest_amount(plan, now)?;
    plan.last_claim_time = now;
    plan.interest_accrued = plan
        .interest_accrued
        .checked_add(interest)
        .ok_or(ErrorCode::MathOverflow)?;

    if interest == 0 {
        return Ok(0);
    }

    require!(
        interest_vault_token_account.amount >= interest,
        ErrorCode::InsufficientFunds
    );

    let signer_seeds: &[&[u8]] = &[
        InterestVault::SEED,
        factory_key.as_ref(),
        &[interest_vault.bump],
    ];
    let signer = &[signer_seeds];

    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        Transfer {
            from: interest_vault_token_account.to_account_info(),
            to: recipient.to_account_info(),
            authority: interest_vault.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi_ctx, interest)?;

    interest_vault.total_claimed = interest_vault
        .total_claimed
        .checked_add(interest)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(interest)
}

fn calculate_interest_amount(plan: &SavingsPlan, now: i64) -> Result<u64> {
    if now <= plan.last_claim_time {
        return Ok(0);
    }
    let elapsed = now
        .checked_sub(plan.last_claim_time)
        .ok_or(ErrorCode::MathOverflow)? as u128;

    let rate = plan.interest_rate_basis_points as u128;
    let principal = plan.principal_amount as u128;

    let numerator = principal
        .checked_mul(rate)
        .and_then(|value| value.checked_mul(elapsed))
        .ok_or(ErrorCode::MathOverflow)?;

    let denominator = (SECONDS_PER_YEAR as u128)
        .checked_mul(10_000)
        .ok_or(ErrorCode::MathOverflow)?;

    let interest = numerator
        .checked_div(denominator)
        .ok_or(ErrorCode::MathOverflow)?;

    let interest_u64 = u64::try_from(interest).map_err(|_| ErrorCode::MathOverflow)?;
    Ok(interest_u64)
}

fn transfer_from_plan_vault<'info>(
    amount: u64,
    plan: &Account<'info, SavingsPlan>,
    plan_vault: &Account<'info, TokenAccount>,
    destination: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let plan_index_bytes = plan.plan_index.to_le_bytes();
    let seeds: &[&[u8]] = &[
        SavingsPlan::SEED,
        plan.owner.as_ref(),
        &plan_index_bytes,
        &[plan.bump],
    ];
    let signer_seeds = &[seeds];

    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        Transfer {
            from: plan_vault.to_account_info(),
            to: destination.to_account_info(),
            authority: plan.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

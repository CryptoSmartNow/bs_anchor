use borsh::{to_vec, BorshDeserialize};
use bitsave::{
    instruction::BitsaveInstruction,
    pda,
    processor::Processor,
    state::{SavingsPlan, UserProfile},
};
use solana_program::{instruction::{AccountMeta, Instruction}, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::ReadableAccount,
    signature::{Keypair, Signer},
    system_program,
    sysvar::clock::Clock,
    transaction::Transaction,
};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};
use spl_token::{
    instruction as token_instruction,
    state::Account as TokenAccount,
};

const REGISTRATION_FEE: u64 = 1_000_000;
const CREATION_FEE: u64 = 1_000_000;

#[tokio::test]
async fn registration_create_topup_and_early_withdraw_work() {
    let mut ctx = setup_context().await;
    let payer_pubkey = ctx.payer.pubkey();

    let treasury = Keypair::new();
    let buyback = Keypair::new();
    let user = Keypair::new();

    fund_wallet(&mut ctx, &treasury.pubkey(), 2_000_000_000).await;
    fund_wallet(&mut ctx, &buyback.pubkey(), 2_000_000_000).await;
    fund_wallet(&mut ctx, &user.pubkey(), 2_000_000_000).await;

    let payer = ctx.payer.insecure_clone();
    let usdc_mint = create_mint(&mut ctx, &payer, 6).await;
    let usdt_mint = create_mint(&mut ctx, &payer, 6).await;

    let user_usdc = create_ata(&mut ctx, &payer, &user.pubkey(), &usdc_mint).await;
    let treasury_usdc = create_ata(&mut ctx, &payer, &treasury.pubkey(), &usdc_mint).await;
    let buyback_usdc = create_ata(&mut ctx, &payer, &buyback.pubkey(), &usdc_mint).await;

    mint_to(&mut ctx, &payer, &usdc_mint, &user_usdc, 100_000_000).await;

    let (factory_pda, _) = pda::factory_pda(&bitsave::id());
    let init_ix = Instruction {
        program_id: bitsave::id(),
        accounts: vec![
            AccountMeta::new(payer_pubkey, true),
            AccountMeta::new_readonly(treasury.pubkey(), false),
            AccountMeta::new_readonly(buyback.pubkey(), false),
            AccountMeta::new(factory_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: to_vec(&BitsaveInstruction::InitializeFactory {
            registration_fee: REGISTRATION_FEE,
            savings_creation_fee: CREATION_FEE,
            supported_stablecoins: vec![usdc_mint, usdt_mint],
        })
        .unwrap(),
    };
    process_tx(&mut ctx, &[init_ix], &[]).await;

    let (user_profile_pda, _) = pda::user_profile_pda(&bitsave::id(), &user.pubkey());
    let register_ix = Instruction {
        program_id: bitsave::id(),
        accounts: vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(factory_pda, false),
            AccountMeta::new(user_profile_pda, false),
            AccountMeta::new(user_usdc, false),
            AccountMeta::new(treasury_usdc, false),
            AccountMeta::new(buyback_usdc, false),
            AccountMeta::new_readonly(usdc_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: to_vec(&BitsaveInstruction::RegisterUser).unwrap(),
    };
    process_tx(&mut ctx, &[register_ix], &[&user]).await;

    let profile = get_state::<UserProfile>(&mut ctx, user_profile_pda).await;
    assert!(profile.is_initialized);

    let (plan_pda, _) = pda::savings_plan_pda(&bitsave::id(), &user.pubkey(), 0);
    let plan_vault = create_ata(&mut ctx, &payer, &plan_pda, &usdc_mint).await;

    let create_ix = Instruction {
        program_id: bitsave::id(),
        accounts: vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(factory_pda, false),
            AccountMeta::new(user_profile_pda, false),
            AccountMeta::new(plan_pda, false),
            AccountMeta::new(plan_vault, false),
            AccountMeta::new(user_usdc, false),
            AccountMeta::new(treasury_usdc, false),
            AccountMeta::new(buyback_usdc, false),
            AccountMeta::new_readonly(usdc_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: to_vec(&BitsaveInstruction::CreateSavingsPlan {
            name: "Vacation Fund".to_string(),
            amount: 10_000_000,
            lock_duration_seconds: 120,
            penalty_rate: 2,
        })
        .unwrap(),
    };
    process_tx(&mut ctx, &[create_ix], &[&user]).await;

    let top_up_ix = Instruction {
        program_id: bitsave::id(),
        accounts: vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(user_profile_pda, false),
            AccountMeta::new(plan_pda, false),
            AccountMeta::new(plan_vault, false),
            AccountMeta::new(user_usdc, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: to_vec(&BitsaveInstruction::TopUpSavings {
            plan_index: 0,
            additional_amount: 5_000_000,
        })
        .unwrap(),
    };
    process_tx(&mut ctx, &[top_up_ix], &[&user]).await;

    let plan = get_state::<SavingsPlan>(&mut ctx, plan_pda).await;
    assert_eq!(plan.principal_amount, 15_000_000);

    let treasury_before = token_amount(&mut ctx, treasury_usdc).await;
    let buyback_before = token_amount(&mut ctx, buyback_usdc).await;
    let user_before = token_amount(&mut ctx, user_usdc).await;

    let withdraw_ix = Instruction {
        program_id: bitsave::id(),
        accounts: vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(user_profile_pda, false),
            AccountMeta::new(factory_pda, false),
            AccountMeta::new(plan_pda, false),
            AccountMeta::new(plan_vault, false),
            AccountMeta::new(user_usdc, false),
            AccountMeta::new(treasury_usdc, false),
            AccountMeta::new(buyback_usdc, false),
            AccountMeta::new_readonly(usdc_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: to_vec(&BitsaveInstruction::WithdrawSavings { plan_index: 0 }).unwrap(),
    };
    process_tx(&mut ctx, &[withdraw_ix], &[&user]).await;

    let expected_penalty = 300_000;
    let expected_return = 14_700_000;
    assert_eq!(token_amount(&mut ctx, user_usdc).await - user_before, expected_return);
    assert_eq!(token_amount(&mut ctx, treasury_usdc).await - treasury_before, expected_penalty / 2);
    assert_eq!(token_amount(&mut ctx, buyback_usdc).await - buyback_before, expected_penalty / 2);
}

#[tokio::test]
async fn mature_withdraw_returns_full_principal() {
    let mut ctx = setup_context().await;
    let payer_pubkey = ctx.payer.pubkey();

    let treasury = Keypair::new();
    let buyback = Keypair::new();
    let user = Keypair::new();

    fund_wallet(&mut ctx, &treasury.pubkey(), 2_000_000_000).await;
    fund_wallet(&mut ctx, &buyback.pubkey(), 2_000_000_000).await;
    fund_wallet(&mut ctx, &user.pubkey(), 2_000_000_000).await;

    let payer = ctx.payer.insecure_clone();
    let usdc_mint = create_mint(&mut ctx, &payer, 6).await;
    let usdt_mint = create_mint(&mut ctx, &payer, 6).await;
    let user_usdc = create_ata(&mut ctx, &payer, &user.pubkey(), &usdc_mint).await;
    let user_usdt = create_ata(&mut ctx, &payer, &user.pubkey(), &usdt_mint).await;
    let treasury_usdc = create_ata(&mut ctx, &payer, &treasury.pubkey(), &usdc_mint).await;
    let buyback_usdc = create_ata(&mut ctx, &payer, &buyback.pubkey(), &usdc_mint).await;
    let treasury_usdt = create_ata(&mut ctx, &payer, &treasury.pubkey(), &usdt_mint).await;
    let buyback_usdt = create_ata(&mut ctx, &payer, &buyback.pubkey(), &usdt_mint).await;

    mint_to(&mut ctx, &payer, &usdc_mint, &user_usdc, 10_000_000).await;
    mint_to(&mut ctx, &payer, &usdt_mint, &user_usdt, 50_000_000).await;

    let (factory_pda, _) = pda::factory_pda(&bitsave::id());
    process_tx(
        &mut ctx,
        &[Instruction {
            program_id: bitsave::id(),
            accounts: vec![
                AccountMeta::new(payer_pubkey, true),
                AccountMeta::new_readonly(treasury.pubkey(), false),
                AccountMeta::new_readonly(buyback.pubkey(), false),
                AccountMeta::new(factory_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: to_vec(&BitsaveInstruction::InitializeFactory {
                registration_fee: REGISTRATION_FEE,
                savings_creation_fee: CREATION_FEE,
                supported_stablecoins: vec![usdc_mint, usdt_mint],
            })
            .unwrap(),
        }],
        &[],
    )
    .await;

    let (user_profile_pda, _) = pda::user_profile_pda(&bitsave::id(), &user.pubkey());
    process_tx(
        &mut ctx,
        &[Instruction {
            program_id: bitsave::id(),
            accounts: vec![
                AccountMeta::new(user.pubkey(), true),
                AccountMeta::new(factory_pda, false),
                AccountMeta::new(user_profile_pda, false),
                AccountMeta::new(user_usdc, false),
                AccountMeta::new(treasury_usdc, false),
                AccountMeta::new(buyback_usdc, false),
                AccountMeta::new_readonly(usdc_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: to_vec(&BitsaveInstruction::RegisterUser).unwrap(),
        }],
        &[&user],
    )
    .await;

    let (plan_pda, _) = pda::savings_plan_pda(&bitsave::id(), &user.pubkey(), 0);
    let plan_vault = create_ata(&mut ctx, &payer, &plan_pda, &usdt_mint).await;

    process_tx(
        &mut ctx,
        &[Instruction {
            program_id: bitsave::id(),
            accounts: vec![
                AccountMeta::new(user.pubkey(), true),
                AccountMeta::new(factory_pda, false),
                AccountMeta::new(user_profile_pda, false),
                AccountMeta::new(plan_pda, false),
                AccountMeta::new(plan_vault, false),
                AccountMeta::new(user_usdt, false),
                AccountMeta::new(treasury_usdt, false),
                AccountMeta::new(buyback_usdt, false),
                AccountMeta::new_readonly(usdt_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: to_vec(&BitsaveInstruction::CreateSavingsPlan {
                name: "Emergency".to_string(),
                amount: 20_000_000,
                lock_duration_seconds: 1,
                penalty_rate: 5,
            })
            .unwrap(),
        }],
        &[&user],
    )
    .await;

    let mut clock = ctx.banks_client.get_sysvar::<Clock>().await.unwrap();
        clock.unix_timestamp += 10;
    ctx.set_sysvar(&clock);

    let before = token_amount(&mut ctx, user_usdt).await;
    process_tx(
        &mut ctx,
        &[Instruction {
            program_id: bitsave::id(),
            accounts: vec![
                AccountMeta::new(user.pubkey(), true),
                AccountMeta::new(user_profile_pda, false),
                AccountMeta::new(factory_pda, false),
                AccountMeta::new(plan_pda, false),
                AccountMeta::new(plan_vault, false),
                AccountMeta::new(user_usdt, false),
                AccountMeta::new(treasury_usdt, false),
                AccountMeta::new(buyback_usdt, false),
                AccountMeta::new_readonly(usdt_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data: to_vec(&BitsaveInstruction::WithdrawSavings { plan_index: 0 }).unwrap(),
        }],
        &[&user],
    )
    .await;
    assert_eq!(token_amount(&mut ctx, user_usdt).await - before, 20_000_000);
}

async fn setup_context() -> ProgramTestContext {
    ProgramTest::new("bitsave", bitsave::id(), processor!(Processor::process))
        .start_with_context()
        .await
}

async fn process_tx(ctx: &mut ProgramTestContext, instructions: &[Instruction], signers: &[&Keypair]) {
    let mut all_signers = vec![&ctx.payer];
    all_signers.extend_from_slice(signers);
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&ctx.payer.pubkey()),
        &all_signers,
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
}

async fn fund_wallet(ctx: &mut ProgramTestContext, to: &Pubkey, lamports: u64) {
    let ix = solana_sdk::system_instruction::transfer(&ctx.payer.pubkey(), to, lamports);
    process_tx(ctx, &[ix], &[]).await;
}

async fn create_mint(ctx: &mut ProgramTestContext, payer: &Keypair, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let rent = ctx.banks_client.get_rent().await.unwrap().minimum_balance(spl_token::state::Mint::LEN);
    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_ix = token_instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &[payer, &mint],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
    mint.pubkey()
}

async fn create_ata(
    ctx: &mut ProgramTestContext,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);
    let ix = create_associated_token_account(&payer.pubkey(), owner, mint, &spl_token::id());
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
    ata
}

async fn mint_to(
    ctx: &mut ProgramTestContext,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    amount: u64,
) {
    let ix = token_instruction::mint_to(
        &spl_token::id(),
        mint,
        destination,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
}

async fn token_amount(ctx: &mut ProgramTestContext, account: Pubkey) -> u64 {
    let account = ctx.banks_client.get_account(account).await.unwrap().unwrap();
    TokenAccount::unpack(account.data()).unwrap().amount
}

async fn get_state<T: BorshDeserialize>(ctx: &mut ProgramTestContext, address: Pubkey) -> T {
    let account = ctx.banks_client.get_account(address).await.unwrap().unwrap();
    let mut slice: &[u8] = account.data();
    T::deserialize(&mut slice).unwrap()
}

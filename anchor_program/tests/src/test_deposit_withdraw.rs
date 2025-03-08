use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
        signature::read_keypair_file, signer::Signer, system_program,
    },
    Client, Cluster,
};
use std::str::FromStr;

#[test]
fn test_deposit_withdraw() {
    // Setup - handle errors manually to avoid thread safety issues
    let program_id_str = "Hai1ivWmZHQD9aWuVzDQSGovam7p3ttdsFTmmiTVvAvB";
    let anchor_wallet = std::env::var("ANCHOR_WALLET").expect("Failed to get ANCHOR_WALLET");
    let payer = read_keypair_file(&anchor_wallet).expect("Failed to read keypair file");

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(program_id_str).expect("Invalid program ID");
    let program = client.program(program_id).expect("Failed to get program");

    // Get initial balance using RPC client
    let rpc_client = program.rpc();

    // Derive the user account PDA
    let user_pubkey = payer.pubkey();
    let seeds = [user_pubkey.as_ref()];
    let (user_account_pda, _) = Pubkey::find_program_address(&seeds, &program_id);
    // Derive the vault PDA
    let vault_seed = b"vault";
    let vault_seeds = [vault_seed.as_ref(), user_pubkey.as_ref()];
    let (vault_pda, _) = Pubkey::find_program_address(&vault_seeds, &program_id);

    // Get vault initial balance
    let vault_initial_balance = match rpc_client.get_account(&vault_pda) {
        Ok(account) => account.lamports,
        Err(_) => 0,
    };
    println!(
        "Vault initial balance: {} SOL",
        vault_initial_balance as f64 / LAMPORTS_PER_SOL as f64
    );

    // Amount to deposit
    let deposit_amount = LAMPORTS_PER_SOL; // 1 SOL

    // Deposit funds
    let tx = program
        .request()
        .accounts(anchor_program::accounts::Deposit {
            user: user_pubkey,
            user_account: user_account_pda,
            vault: vault_pda,
            system_program: system_program::ID,
        })
        .args(anchor_program::instruction::Deposit {
            amount: deposit_amount,
        })
        .send()
        .expect("Failed to deposit");

    println!("\nDeposit transaction signature: {}", tx);

    // Get vault balance after deposit
    let vault_account = rpc_client
        .get_account(&vault_pda)
        .expect("Failed to get vault account");
    let vault_after_deposit = vault_account.lamports;
    println!(
        "Vault balance after deposit: {} SOL",
        vault_after_deposit as f64 / LAMPORTS_PER_SOL as f64
    );
    assert_eq!(
        vault_after_deposit,
        vault_initial_balance + deposit_amount,
        "Vault balance should increase by deposit amount"
    );

    // Get user balance after deposit
    let balance_after_deposit = rpc_client
        .get_balance(&user_pubkey)
        .expect("Failed to get user balance");
    println!(
        "User balance after deposit: {} SOL",
        balance_after_deposit as f64 / LAMPORTS_PER_SOL as f64
    );

    // Now withdraw the funds
    let withdraw_amount = deposit_amount / 2; // Withdraw half of what was deposited

    let tx = program
        .request()
        .accounts(anchor_program::accounts::Withdraw {
            user: user_pubkey,
            user_account: user_account_pda,
            vault: vault_pda,
            system_program: system_program::ID,
        })
        .args(anchor_program::instruction::Withdraw {
            amount: withdraw_amount,
        })
        .send()
        .expect("Failed to withdraw");

    println!("\nWithdraw transaction signature: {}", tx);

    // Get vault balance after withdrawal
    let vault_account = rpc_client
        .get_account(&vault_pda)
        .expect("Failed to get vault account");
    let vault_after_withdraw = vault_account.lamports;
    println!(
        "Vault balance after withdrawal: {} SOL",
        vault_after_withdraw as f64 / LAMPORTS_PER_SOL as f64
    );
    assert_eq!(
        vault_after_withdraw,
        vault_after_deposit - withdraw_amount,
        "Vault balance should decrease by withdraw amount"
    );

    // Get user balance after withdrawal
    let balance_after_withdraw = rpc_client
        .get_balance(&user_pubkey)
        .expect("Failed to get user balance");
    println!(
        "User balance after withdrawal: {} SOL\n",
        balance_after_withdraw as f64 / LAMPORTS_PER_SOL as f64
    );
    assert!(
        balance_after_withdraw > balance_after_deposit,
        "User balance should increase after withdrawal"
    );
}

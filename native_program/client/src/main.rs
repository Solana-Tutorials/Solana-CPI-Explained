use anyhow::Result;
use borsh::BorshSerialize;
use solana_client::rpc_client::RpcClient;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::{str::FromStr, thread, time::Duration};

const PROGRAM_ID_STR: &str = "DPFTib3APrmJaBYjYmVamEpsPiHQ4cSkYLYXiGQmYUja";
const RPC_URL: &str = "http://127.0.0.1:8899";

// Instruction types for serialization
#[derive(Debug, BorshSerialize)]
enum ProgramInstruction {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
}

impl ProgramInstruction {
    fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        BorshSerialize::serialize(self, &mut data).unwrap();
        data
    }
}

fn find_user_account_address(user_pubkey: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[user_pubkey.as_ref()], program_id)
}

fn find_vault_address(user_pubkey: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", user_pubkey.as_ref()], program_id)
}

fn main() -> Result<()> {
    // Create connection
    let commitment_config = CommitmentConfig::confirmed();
    let connection = RpcClient::new_with_commitment(RPC_URL.to_string(), commitment_config);

    // Get the keypair from the default Solana config
    let home = std::env::var("HOME").expect("Failed to get HOME env var");
    let payer_keypair_path = format!("{}/.config/solana/id.json", home);
    let payer = read_keypair_file(&payer_keypair_path).expect("Failed to read keypair file");

    // Get the program ID
    let program_id = Pubkey::from_str(PROGRAM_ID_STR)?;

    // Get the user and vault PDAs
    let user_pubkey = payer.pubkey();
    let (user_account_pda, _) = find_user_account_address(&user_pubkey, &program_id);
    let (vault_pda, _) = find_vault_address(&user_pubkey, &program_id);

    // Get initial balances
    let user_initial_balance = connection.get_balance(&user_pubkey)?;
    let vault_initial_balance = match connection.get_account(&vault_pda) {
        Ok(account) => account.lamports,
        Err(_) => 0,
    };

    println!(
        "User initial balance: {} SOL",
        user_initial_balance as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "Vault initial balance: {} SOL",
        vault_initial_balance as f64 / LAMPORTS_PER_SOL as f64
    );

    // Amount to deposit
    let deposit_amount = LAMPORTS_PER_SOL; // 1 SOL

    // Create deposit instruction
    let instruction_data = ProgramInstruction::Deposit {
        amount: deposit_amount,
    }
    .serialize();

    let deposit_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user_pubkey, true), // User (signer, writable)
            AccountMeta::new(user_account_pda, false), // User account PDA (writable)
            AccountMeta::new(vault_pda, false),  // Vault PDA (writable)
            AccountMeta::new_readonly(system_program::id(), false), // System program
        ],
        data: instruction_data,
    };

    // Send deposit transaction
    let recent_blockhash = connection.get_latest_blockhash()?;
    let deposit_transaction = Transaction::new_signed_with_payer(
        &[deposit_instruction],
        Some(&user_pubkey),
        &[&payer],
        recent_blockhash,
    );

    let deposit_signature = connection.send_and_confirm_transaction(&deposit_transaction)?;
    println!("\nDeposit transaction signature: {}", deposit_signature);

    // Get balances after deposit
    let user_after_deposit = connection.get_balance(&user_pubkey)?;
    let vault_account = connection.get_account(&vault_pda)?;
    let vault_after_deposit = vault_account.lamports;

    println!(
        "User balance after deposit: {} SOL",
        user_after_deposit as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "Vault balance after deposit: {} SOL",
        vault_after_deposit as f64 / LAMPORTS_PER_SOL as f64
    );

    // Wait a bit before withdrawing
    thread::sleep(Duration::from_secs(2));

    // Now withdraw half of what was deposited
    let withdraw_amount = deposit_amount / 2;

    // Create withdraw instruction
    let instruction_data = ProgramInstruction::Withdraw {
        amount: withdraw_amount,
    }
    .serialize();

    let withdraw_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user_pubkey, true), // User (signer, writable)
            AccountMeta::new(user_account_pda, false), // User account PDA (writable)
            AccountMeta::new(vault_pda, false),  // Vault PDA (writable)
            AccountMeta::new_readonly(system_program::id(), false), // System program
        ],
        data: instruction_data,
    };

    // Send withdraw transaction
    let recent_blockhash = connection.get_latest_blockhash()?;
    let withdraw_transaction = Transaction::new_signed_with_payer(
        &[withdraw_instruction],
        Some(&user_pubkey),
        &[&payer],
        recent_blockhash,
    );

    let withdraw_signature = connection.send_and_confirm_transaction(&withdraw_transaction)?;
    println!("\nWithdraw transaction signature: {}", withdraw_signature);

    // Get balances after withdrawal
    let user_after_withdraw = connection.get_balance(&user_pubkey)?;
    let vault_account = connection.get_account(&vault_pda)?;
    let vault_after_withdraw = vault_account.lamports;

    println!(
        "User balance after withdrawal: {} SOL",
        user_after_withdraw as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "Vault balance after withdrawal: {} SOL",
        vault_after_withdraw as f64 / LAMPORTS_PER_SOL as f64
    );

    Ok(())
}

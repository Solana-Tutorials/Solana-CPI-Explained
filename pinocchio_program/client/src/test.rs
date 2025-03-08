// cargo test test_deposit_withdraw -- --nocapture
use anyhow::Result;
use borsh::BorshSerialize;
use borsh_derive::BorshSerialize as BorshSerializeDerive;
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
use std::{env, str::FromStr};

const PROGRAM_ID_STR: &str = "G7isKoAvjaMXi7CSDZTspXvUaD2dfVNwZyrWYTe6nfoj";
const RPC_URL: &str = "http://127.0.0.1:8899";

// Instruction types for serialization
#[derive(Debug, BorshSerializeDerive)]
pub enum ProgramInstruction {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
}

impl ProgramInstruction {
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        BorshSerialize::serialize(self, &mut data).unwrap();
        data
    }
}

// Helper functions for finding PDAs
fn find_user_account_address(user_pubkey: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[user_pubkey.as_ref()], program_id)
}

fn find_vault_address(user_pubkey: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", user_pubkey.as_ref()], program_id)
}

#[test]
fn test_deposit_withdraw() -> Result<()> {
    // Setup - connect to local Solana testnet
    let commitment_config = CommitmentConfig::confirmed();
    let rpc_client = RpcClient::new_with_commitment(RPC_URL.to_string(), commitment_config);

    // Use the default keypair from Solana config for testing
    let home = env::var("HOME").expect("Failed to get HOME env var");
    let payer_keypair_path = format!("{}/.config/solana/id.json", home);
    let payer = read_keypair_file(&payer_keypair_path).expect("Failed to read keypair file");

    // Get the program ID from the PROGRAM_ID constant
    let program_id = Pubkey::from_str(PROGRAM_ID_STR).expect("Invalid program ID");

    // Find the PDAs for user account and vault
    let user_pubkey = payer.pubkey();
    let (user_account_pda, _) = find_user_account_address(&user_pubkey, &program_id);
    let (vault_pda, _) = find_vault_address(&user_pubkey, &program_id);

    println!("User PDA: {}", user_account_pda);
    println!("Vault PDA: {}", vault_pda);

    // Check if vault account exists
    let vault_exists = rpc_client
        .get_account_with_commitment(&vault_pda, commitment_config)
        .unwrap()
        .value
        .is_some();

    // Get vault initial balance
    let vault_initial_balance = if vault_exists {
        rpc_client.get_balance(&vault_pda)?
    } else {
        0
    };
    println!(
        "Vault initial balance: {} SOL",
        vault_initial_balance as f64 / LAMPORTS_PER_SOL as f64
    );

    // Amount to deposit
    let deposit_amount = LAMPORTS_PER_SOL; // 1 SOL

    // Create deposit instruction using Borsh serialization
    let instruction_data = ProgramInstruction::Deposit {
        amount: deposit_amount,
    }
    .serialize();

    let deposit_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true), // User (signer, writable)
            AccountMeta::new(user_account_pda, false), // User account PDA (writable)
            AccountMeta::new(vault_pda, false),     // Vault PDA (writable)
            AccountMeta::new_readonly(system_program::id(), false), // System program
        ],
        data: instruction_data,
    };

    // Send deposit transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let deposit_transaction = Transaction::new_signed_with_payer(
        &[deposit_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let deposit_signature = rpc_client.send_and_confirm_transaction(&deposit_transaction)?;
    println!("\nDeposit transaction signature: {}", deposit_signature);

    // Get vault balance after deposit
    let vault_after_deposit = rpc_client.get_balance(&vault_pda)?;
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
    let balance_after_deposit = rpc_client.get_balance(&payer.pubkey())?;
    println!(
        "User balance after deposit: {} SOL",
        balance_after_deposit as f64 / LAMPORTS_PER_SOL as f64
    );

    // Now withdraw half of what was deposited
    let withdraw_amount = deposit_amount / 2;

    // Create withdraw instruction using Borsh serialization
    let instruction_data = ProgramInstruction::Withdraw {
        amount: withdraw_amount,
    }
    .serialize();

    let withdraw_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true), // User (signer, writable)
            AccountMeta::new(user_account_pda, false), // User account PDA (readable)
            AccountMeta::new(vault_pda, false),     // Vault PDA (writable)
            AccountMeta::new_readonly(system_program::id(), false), // System program
        ],
        data: instruction_data,
    };

    // Send withdraw transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let withdraw_transaction = Transaction::new_signed_with_payer(
        &[withdraw_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let withdraw_signature = rpc_client.send_and_confirm_transaction(&withdraw_transaction)?;
    println!("\nWithdraw transaction signature: {}", withdraw_signature);

    // Get vault balance after withdrawal
    let vault_after_withdraw = rpc_client.get_balance(&vault_pda)?;
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
    let balance_after_withdraw = rpc_client.get_balance(&payer.pubkey())?;
    println!(
        "User balance after withdrawal: {} SOL\n",
        balance_after_withdraw as f64 / LAMPORTS_PER_SOL as f64
    );
    assert!(
        balance_after_withdraw > balance_after_deposit,
        "User balance should increase after withdrawal"
    );

    Ok(())
}

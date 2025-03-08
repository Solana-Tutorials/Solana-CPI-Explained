// cargo build-sbf
// solana program deploy ./target/deploy/program.so
// solana address -k ./target/deploy/program-keypair.json

#![allow(unexpected_cfgs)]
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

// Declare program entrypoint
entrypoint!(process_instruction);

// Program instruction enum
#[derive(Debug, BorshDeserialize)]
enum ProgramInstruction {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
}

impl ProgramInstruction {
    // Deserialize the instruction data using Borsh
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(input).map_err(|_| ProgramError::InvalidInstructionData)
    }
}

// User account data structure compatible with borsh
#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct UserAccount {
    pub user: Pubkey,
    pub user_bump: u8,
    pub vault_bump: u8,
    pub is_initialized: bool,
}

impl UserAccount {
    const SIZE: usize = 32 + 1 + 1 + 1; // pubkey + user_bump + vault_bump + is_initialized
}

// Main instruction processor
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ProgramInstruction::unpack(instruction_data)?;

    match instruction {
        ProgramInstruction::Deposit { amount } => process_deposit(program_id, accounts, amount),
        ProgramInstruction::Withdraw { amount } => process_withdraw(program_id, accounts, amount),
    }
}

// Process deposit instruction
fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [user_account_info, user_data_account_info, vault_account_info, system_program_account_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check that the user signed the transaction
    if !user_account_info.is_signer {
        msg!("User must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check that the system program is valid
    if system_program_account_info.key != &system_program::id() {
        msg!("Invalid system program");
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive and verify the user data PDA
    let (expected_user_data_pubkey, user_data_bump) =
        Pubkey::find_program_address(&[user_account_info.key.as_ref()], program_id);
    if user_data_account_info.key != &expected_user_data_pubkey {
        msg!("Invalid user data account address");
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive and verify the vault PDA
    let (expected_vault_pubkey, vault_bump) =
        Pubkey::find_program_address(&[b"vault", user_account_info.key.as_ref()], program_id);
    if vault_account_info.key != &expected_vault_pubkey {
        msg!("Invalid vault account address");
        return Err(ProgramError::InvalidAccountData);
    }

    // Initialize user account if needed
    if user_data_account_info.owner != program_id {
        msg!("Creating user data account");
        // Calculate rent
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(UserAccount::SIZE);

        // Create the account
        invoke_signed(
            &system_instruction::create_account(
                user_account_info.key,
                user_data_account_info.key,
                rent_lamports,
                UserAccount::SIZE as u64,
                program_id,
            ),
            &[
                user_account_info.clone(),
                user_data_account_info.clone(),
                system_program_account_info.clone(),
            ],
            &[&[user_account_info.key.as_ref(), &[user_data_bump]]],
        )?;

        // Initialize the account data using borsh
        let user_data = UserAccount {
            user: *user_account_info.key,
            user_bump: user_data_bump,
            vault_bump: vault_bump,
            is_initialized: true,
        };

        user_data.serialize(&mut *user_data_account_info.try_borrow_mut_data()?)?;
    }

    // Transfer lamports to the vault
    invoke(
        &system_instruction::transfer(user_account_info.key, vault_account_info.key, amount),
        &[
            user_account_info.clone(),
            vault_account_info.clone(),
            system_program_account_info.clone(),
        ],
    )?;

    msg!("Deposited {} lamports to vault", amount);

    Ok(())
}

// Process withdraw instruction
fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [user_account_info, user_data_account_info, vault_account_info, system_program_account_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check that the user signed the transaction
    if !user_account_info.is_signer {
        msg!("User must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check that the system program is valid
    if system_program_account_info.key != &system_program::id() {
        msg!("Invalid system program");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify user account data using borsh
    let data = user_data_account_info.try_borrow_data()?;
    let user_data = match UserAccount::try_from_slice(&data) {
        Ok(data) => data,
        Err(_) => {
            msg!("Failed to deserialize user account data");
            return Err(ProgramError::InvalidAccountData);
        }
    };

    // Check that the user account belongs to the requesting user
    if user_data.user != *user_account_info.key {
        msg!("User account does not belong to the requesting user");
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive and user data PDA
    let expected_user_data_pubkey = Pubkey::create_program_address(
        &[user_account_info.key.as_ref(), &[user_data.user_bump]],
        program_id,
    )?;

    if user_data_account_info.key != &expected_user_data_pubkey {
        msg!("Invalid user data account address");
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive and verify the vault PDA
    let expected_vault_pubkey = Pubkey::create_program_address(
        &[
            b"vault",
            user_account_info.key.as_ref(),
            &[user_data.vault_bump],
        ],
        program_id,
    )?;

    if vault_account_info.key != &expected_vault_pubkey {
        msg!("Invalid vault account address");
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive and verify the vault PDA
    let signer_seeds = [
        b"vault",
        user_account_info.key.as_ref(),
        &[user_data.vault_bump],
    ];

    // Create the transfer instruction
    let transfer_instruction = system_instruction::transfer(
        &vault_account_info.key, // from
        &user_account_info.key,  // to
        amount,                  // amount
    );

    // Execute the transfer with the vault's PDA authority
    invoke_signed(
        &transfer_instruction,
        &[
            vault_account_info.clone(),
            user_account_info.clone(),
            system_program_account_info.clone(),
        ],
        &[&signer_seeds],
    )?;

    msg!("Withdrew {} lamports from vault", amount);

    Ok(())
}

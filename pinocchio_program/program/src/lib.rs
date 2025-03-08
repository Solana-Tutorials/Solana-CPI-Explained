// cargo build-sbf
// solana program deploy ./target/deploy/pinocchio_program.so
// solana address -k ./target/deploy/pinocchio_program-keypair.json

#![allow(unexpected_cfgs)]
use borsh::{BorshDeserialize, BorshSerialize};
use borsh_derive::{
    BorshDeserialize as BorshDeserializeDerive, BorshSerialize as BorshSerializeDerive,
};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    instruction::{Seed, Signer},
    msg,
    program_error::ProgramError,
    pubkey,
    pubkey::Pubkey,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::{CreateAccount, Transfer};
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;

// Declare program entrypoint
entrypoint!(process_instruction);

// Program instruction enum
#[derive(Debug, BorshDeserializeDerive)]
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
#[derive(Debug, BorshSerializeDerive, BorshDeserializeDerive)]
struct UserAccount {
    pub user: Pubkey,
    pub user_bump: u8,
    pub vault_bump: u8,
    pub is_initialized: bool,
}

impl UserAccount {
    const SIZE: usize = 32 + 1 + 1 + 1; // pubkey + user_bump + vault_bump + is_initialized
}

pub fn process_instruction(
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
    // We expect 4 accounts: user, user_data, vault, system_program
    let [user_account_info, user_data_account_info, vault_account_info, system_program_account_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check that the user signed the transaction
    if !user_account_info.is_signer() {
        msg!("User must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check that the system program is valid
    if system_program_account_info.key() != &SYSTEM_PROGRAM_ID {
        msg!("Invalid system program");
        return Err(ProgramError::InvalidAccountData);
    }

    // Find vault address and bump
    let vault_seeds = &[b"vault", user_account_info.key().as_ref()];
    let (expected_vault_pubkey, vault_bump) = pubkey::find_program_address(vault_seeds, program_id);

    // Verify vault address
    if vault_account_info.key() != &expected_vault_pubkey {
        msg!("Invalid vault account address");
        return Err(ProgramError::InvalidAccountData);
    }

    // Initialize user data account if needed
    if user_data_account_info.owner() != program_id {
        // Calculate rent for account
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(UserAccount::SIZE);

        // Create user data account using system program
        let user_key_bytes = user_account_info.key().as_ref();
        let user_seeds = &[user_key_bytes];
        let (expected_user_data_pubkey, user_bump) =
            pubkey::find_program_address(user_seeds, program_id);

        // Check that provided user data account matches expected PDA
        if user_data_account_info.key() != &expected_user_data_pubkey {
            msg!("Invalid user data account address");
            return Err(ProgramError::InvalidAccountData);
        }

        // Create seeds for PDA signing
        let bump_bytes = [user_bump];
        let seed1 = Seed::from(user_key_bytes);
        let seed2 = Seed::from(&bump_bytes);
        let seeds = [seed1, seed2];
        let signer = Signer::from(&seeds);

        // Create the account
        CreateAccount {
            from: user_account_info,
            to: user_data_account_info,
            lamports: rent_lamports,
            space: UserAccount::SIZE as u64,
            owner: program_id,
        }
        .invoke_signed(&[signer])?;

        // Initialize user data account with vault info
        let user_data = UserAccount {
            user: *user_account_info.key(),
            user_bump,
            vault_bump,
            is_initialized: true,
        };

        // Serialize directly to the account data
        let mut data = user_data_account_info.try_borrow_mut_data()?;
        user_data
            .serialize(&mut &mut data[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    }

    // Transfer lamports to the vault using pinocchio_system
    Transfer {
        from: user_account_info,
        to: vault_account_info,
        lamports: amount,
    }
    .invoke()?;

    msg!("Deposited to vault");

    Ok(())
}

// Process withdraw instruction
fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    // We expect 4 accounts: user, user_data, vault, system_program
    let [user_account_info, user_data_account_info, vault_account_info, system_program_account_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check that the user signed the transaction
    if !user_account_info.is_signer() {
        msg!("User must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check that the system program is valid
    if system_program_account_info.key() != &SYSTEM_PROGRAM_ID {
        msg!("Invalid system program");
        return Err(ProgramError::InvalidAccountData);
    }

    // Read user data account
    let data = user_data_account_info.try_borrow_data()?;
    let user_data =
        UserAccount::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)?;

    // Check that the user account belongs to the requesting user
    if user_data.user != *user_account_info.key() {
        msg!("User account does not belong to the requesting user");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify vault PDA
    let vault_seeds = &[
        b"vault".as_ref(),
        user_account_info.key().as_ref(),
        &[user_data.vault_bump],
    ];

    let expected_vault = match pubkey::create_program_address(vault_seeds, program_id) {
        Ok(address) => address,
        Err(_) => return Err(ProgramError::InvalidAccountData),
    };

    if vault_account_info.key() != &expected_vault {
        msg!("Invalid vault account address");
        return Err(ProgramError::InvalidAccountData);
    }

    // Create seeds for PDA signing
    let vault_bump_bytes = [user_data.vault_bump];
    let seed1 = Seed::from(b"vault");
    let seed2 = Seed::from(user_account_info.key().as_ref());
    let seed3 = Seed::from(&vault_bump_bytes);
    let seeds = [seed1, seed2, seed3];
    let signer = Signer::from(&seeds);

    // Transfer lamports from vault to user with PDA signing
    Transfer {
        from: vault_account_info,
        to: user_account_info,
        lamports: amount,
    }
    .invoke_signed(&[signer])?;

    msg!("Withdrew from vault");

    Ok(())
}

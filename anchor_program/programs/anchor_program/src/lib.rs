#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("Hai1ivWmZHQD9aWuVzDQSGovam7p3ttdsFTmmiTVvAvB");

#[program]
pub mod anchor_program {
    use super::*;

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // Create or update user account data
        let user_account = &mut ctx.accounts.user_account;

        // Initialize if this is the first time
        if !user_account.is_initialized {
            user_account.user = ctx.accounts.user.key();
            user_account.user_bump = ctx.bumps.user_account;
            user_account.vault_bump = ctx.bumps.vault;
            user_account.is_initialized = true;
        }

        // Transfer lamports to the vault
        let cpi_accounts = system_program::Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.system_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        system_program::transfer(cpi_ctx, amount)?;
        msg!("Deposited {} lamports to vault", amount);

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // Check if the vault has enough lamports
        let vault_lamports = ctx.accounts.vault.lamports();
        require!(vault_lamports >= amount, VaultError::InsufficientFunds);

        // Create the vault signer seeds
        let user_key = ctx.accounts.user.key();
        let seeds = [
            b"vault".as_ref(),
            user_key.as_ref(),
            &[ctx.accounts.user_account.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // Transfer lamports from the vault to the user via CPI with signer seeds
        let cpi_accounts = system_program::Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.system_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts).with_signer(signer_seeds);

        system_program::transfer(cpi_ctx, amount)?;

        msg!("Withdrew {} lamports from vault", amount);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserAccount::INIT_SPACE,
        seeds = [user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [b"vault", user.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [user.key().as_ref()],
        bump = user_account.user_bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [b"vault", user.key().as_ref()],
        bump = user_account.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    pub user: Pubkey,         // 32 bytes
    pub user_bump: u8,        // 1 byte
    pub vault_bump: u8,       // 1 byte
    pub is_initialized: bool, // 1 byte
}

#[error_code]
pub enum VaultError {
    #[msg("Insufficient funds in the vault")]
    InsufficientFunds,
}

use anchor_lang::prelude::*; 
use anchor_spl::token::{self, Token, TokenAccount, Transfer}; 

declare_id!("REPLACE_WITH_YOUR_PROGRAM_ID");

#[program]
pub mod glaucoin_contract {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        total_supply: u64,
        initial_release: u64,
        release_interval: i64,
    ) -> Result<()> {
        let my_account = &mut ctx.accounts.my_account;

        my_account.total_supply = total_supply;
        my_account.initial_release = initial_release;
        my_account.remaining_supply = total_supply - initial_release;
        my_account.last_release_time = Clock::get()?.unix_timestamp;
        my_account.release_interval = release_interval;
        my_account.authority = ctx.accounts.authority.key();

        Ok(())
    }

    pub fn auto_release(ctx: Context<AutoRelease>, release_amount: u64) -> Result<()> {
        let my_account = &mut ctx.accounts.my_account;
        let clock = Clock::get()?;

        if clock.unix_timestamp < my_account.last_release_time + my_account.release_interval {
            return Err(ErrorCode::ReleaseTooSoon.into());
        }

        if my_account.remaining_supply < release_amount {
            return Err(ErrorCode::InsufficientSupply.into());
        }

        my_account.remaining_supply -= release_amount;
        my_account.last_release_time = clock.unix_timestamp;

        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account.to_account_info(),
            to: ctx.accounts.receiver_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, release_amount)?;

        Ok(())
    }

    pub fn transfer(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        let source = &ctx.accounts.source;
        let destination = &ctx.accounts.destination;
        let commission_account = &ctx.accounts.commission_account;

        if source.amount < amount {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        let commission = amount / 400;
        let net_amount = amount - commission;

        let cpi_accounts_transfer = Transfer {
            from: source.to_account_info(),
            to: destination.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx_transfer = CpiContext::new(cpi_program.clone(), cpi_accounts_transfer);
        token::transfer(cpi_ctx_transfer, net_amount)?;

        let cpi_accounts_commission = Transfer {
            from: source.to_account_info(),
            to: commission_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx_commission = CpiContext::new(cpi_program.clone(), cpi_accounts_commission);
        token::transfer(cpi_ctx_commission, commission)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 32 + 8 + 8 + 8 + 8 + 8)]
    pub my_account: Account<'info, MyAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AutoRelease<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyAccount>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub receiver_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub source: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,
    #[account(mut)]
    pub commission_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub authority: Signer<'info>,
}

#[account]
pub struct MyAccount {
    pub total_supply: u64,
    pub initial_release: u64,
    pub remaining_supply: u64,
    pub last_release_time: i64,
    pub release_interval: i64,
    pub authority: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Interval not yet reached.")]
    ReleaseTooSoon,
    #[msg("Insufficient balance.")]
    InsufficientFunds,
    #[msg("Not enough tokens available.")]
    InsufficientSupply,
}

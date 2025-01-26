// Importação das bibliotecas necessárias
use anchor_lang::prelude::*; // Facilita o desenvolvimento de contratos na Solana com Anchor
use anchor_spl::token::{self, Token, TokenAccount, Transfer}; // Gerenciamento de tokens SPL na Solana

// Declaração do programa
declare_id!("REPLACE_WITH_YOUR_PROGRAM_ID");

#[program]
pub mod glaucoin_contract {
    use super::*;

    /// Inicializa o contrato com os valores fornecidos.
    pub fn initialize(
        ctx: Context<Initialize>,
        total_supply: u64,
        initial_release: u64,
        release_interval: i64,
    ) -> Result<()> {
        let my_account = &mut ctx.accounts.my_account;

        // Define os valores iniciais do contrato
        my_account.total_supply = total_supply;
        my_account.initial_release = initial_release;
        my_account.remaining_supply = total_supply - initial_release;
        my_account.last_release_time = Clock::get()?.unix_timestamp;
        my_account.release_interval = release_interval;
        my_account.authority = ctx.accounts.authority.key();

        Ok(())
    }

    /// Liberação programada de tokens.
    pub fn auto_release(ctx: Context<AutoRelease>, release_amount: u64) -> Result<()> {
        let my_account = &mut ctx.accounts.my_account;
        let clock = Clock::get()?;

        // Verifica se o intervalo de liberação foi respeitado
        if clock.unix_timestamp < my_account.last_release_time + my_account.release_interval {
            return Err(ErrorCode::ReleaseTooSoon.into());
        }

        // Verifica se há tokens suficientes para liberar
        if my_account.remaining_supply < release_amount {
            return Err(ErrorCode::InsufficientSupply.into());
        }

        // Libera os tokens
        my_account.remaining_supply -= release_amount;
        my_account.last_release_time = clock.unix_timestamp;

        // Realiza a transferência para a conta de destino
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

    /// Transfere tokens entre contas, calculando uma comissão de 0,25%.
    pub fn transfer(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        let source = &ctx.accounts.source;
        let destination = &ctx.accounts.destination;
        let commission_account = &ctx.accounts.commission_account;

        // Verifica se há saldo suficiente na conta de origem
        if source.amount < amount {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        // Calcula a comissão de 0,25% e ajusta o valor transferido
        let commission = amount / 400;
        let net_amount = amount - commission;

        // Transfere o valor líquido para o destinatário
        let cpi_accounts_transfer = Transfer {
            from: source.to_account_info(),
            to: destination.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx_transfer = CpiContext::new(cpi_program.clone(), cpi_accounts_transfer);
        token::transfer(cpi_ctx_transfer, net_amount)?;

        // Transfere a comissão para a conta do criador do contrato
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

// Define os contextos das funções

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 32 + 8 + 8 + 8 + 8 + 8)]
    pub my_account: Account<'info, MyAccount>, // Conta do contrato
    #[account(mut)]
    pub payer: Signer<'info>, // Usuário que paga pela inicialização
    pub authority: Signer<'info>, // Autoridade do contrato
    pub system_program: Program<'info, System>, // Programa do sistema Solana
}

#[derive(Accounts)]
pub struct AutoRelease<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyAccount>, // Conta do contrato
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>, // Conta de onde os tokens serão liberados
    #[account(mut)]
    pub receiver_account: Account<'info, TokenAccount>, // Conta que receberá os tokens liberados
    pub token_program: Program<'info, Token>, // Programa de token SPL
    pub authority: Signer<'info>, // Autoridade que libera os tokens
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub source: Account<'info, TokenAccount>, // Conta de origem
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>, // Conta de destino
    #[account(mut)]
    pub commission_account: Account<'info, TokenAccount>, // Conta que recebe a comissão
    pub token_program: Program<'info, Token>, // Programa de token SPL
    pub authority: Signer<'info>, // Autoridade que autoriza a transferência
}

// Estrutura de dados armazenada na conta do contrato
#[account]
pub struct MyAccount {
    pub total_supply: u64, // Total de tokens emitidos
    pub initial_release: u64, // Quantidade inicial liberada
    pub remaining_supply: u64, // Quantidade restante de tokens
    pub last_release_time: i64, // Timestamp da última liberação
    pub release_interval: i64, // Intervalo de tempo entre liberações
    pub authority: Pubkey, // Autoridade do contrato
}

// Define os erros personalizados
#[error_code]
pub enum ErrorCode {
    #[msg("Intervalo de liberação ainda não foi atingido.")]
    ReleaseTooSoon,
    #[msg("Saldo insuficiente.")]
    InsufficientFunds,
    #[msg("Quantidade insuficiente de tokens para liberar.")]
    InsufficientSupply,
}

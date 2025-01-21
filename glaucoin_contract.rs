use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("YOUR_PROGRAM_ID");

#[program]
pub mod glaucoin {
    use super::*;

    // Inicializar o contrato e definir os parâmetros iniciais
    pub fn initialize(ctx: Context<Initialize>, total_supply: u64, initial_release: u64) -> ProgramResult {
        let token_account = &mut ctx.accounts.token_account;

        // Verificar se o initial_release é válido
        require!(
            initial_release <= total_supply,
            CustomError::InvalidInitialRelease
        );

        let clock = Clock::get()?;
        
        // Configuração inicial do contrato
        token_account.total_supply = total_supply;
        token_account.released_tokens = initial_release;
        token_account.admin = *ctx.accounts.admin.key;
        token_account.airdrop_tokens = 50_000_000; // Para airdrops
        token_account.next_release = Some(clock.unix_timestamp + 30 * 24 * 60 * 60); // Primeiro desbloqueio em 30 dias
        token_account.releases_remaining = 3; // 3 liberações adicionais
        token_account.commission_rate = 0.0025; // Comissão inicial de 0,25%
        Ok(())
    }

    // Função para liberar tokens automaticamente com base no tempo
    pub fn auto_release(ctx: Context<AutoRelease>) -> ProgramResult {
        let token_account = &mut ctx.accounts.token_account;
        let clock = Clock::get()?;

        // Verificar se há liberações restantes
        require!(
            token_account.releases_remaining > 0,
            CustomError::NoReleasesRemaining
        );

        // Verificar se já passou o tempo para a próxima liberação
        require!(
            Some(clock.unix_timestamp) >= token_account.next_release,
            CustomError::ReleaseNotDue
        );

        // Atualizar valores
        token_account.released_tokens += 500_000_000;
        token_account.releases_remaining -= 1;
        token_account.next_release = Some(clock.unix_timestamp + 30 * 24 * 60 * 60);

        emit!(ReleaseLog {
            admin: token_account.admin,
            released_tokens: 500_000_000,
            remaining_releases: token_account.releases_remaining,
        });

        Ok(())
    }

    // Transferir tokens com comissão e limite de volume
    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> ProgramResult {
        let sender = &mut ctx.accounts.sender;
        let recipient = &mut ctx.accounts.recipient;
        let fund_account = &mut ctx.accounts.fund_account;

        // Impedir transferências acima de 10 milhões (proteção contra "baleias")
        require!(amount <= 10_000_000, CustomError::WhaleTransferRestricted);

        // Calcular a comissão
        let commission = calculate_commission(amount, ctx.accounts.token_account.commission_rate);
        let amount_after_commission = amount - commission;

        // Verificar saldo suficiente
        require!(sender.balance >= amount, CustomError::InsufficientBalance);

        // Transferir tokens
        sender.balance -= amount;
        recipient.balance += amount_after_commission;
        fund_account.balance += commission;

        emit!(TransferLog {
            sender: *sender.key,
            recipient: *recipient.key,
            amount: amount_after_commission,
            commission,
        });

        Ok(())
    }

    // Atualizar a taxa de comissão
    pub fn update_commission_rate(ctx: Context<AdminOnly>, new_rate: f64) -> ProgramResult {
        let token_account = &mut ctx.accounts.token_account;

        // Verificar se o administrador é quem está executando a alteração
        require!(
            *ctx.accounts.admin.key == token_account.admin,
            CustomError::Unauthorized
        );

        // Atualizar a taxa de comissão
        token_account.commission_rate = new_rate;

        emit!(UpdateCommissionRateLog {
            admin: *ctx.accounts.admin.key,
            new_rate,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = 8 + 128)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AutoRelease<'info> {
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(signer)]
    pub admin: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub sender: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(signer)]
    pub admin: AccountInfo<'info>,
}

#[account]
pub struct TokenAccount {
    pub total_supply: u64,
    pub released_tokens: u64,
    pub admin: Pubkey,
    pub airdrop_tokens: u64,
    pub next_release: Option<i64>, // Timestamp do próximo desbloqueio
    pub releases_remaining: u8,   // Número de liberações restantes
    pub commission_rate: f64,     // Taxa de comissão
    pub balance: u64,
}

#[event]
pub struct TransferLog {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub commission: u64,
}

#[event]
pub struct ReleaseLog {
    pub admin: Pubkey,
    pub released_tokens: u64,
    pub remaining_releases: u8,
}

#[event]
pub struct UpdateCommissionRateLog {
    pub admin: Pubkey,
    pub new_rate: f64,
}

#[error]
pub enum CustomError {
    #[msg("Você não tem permissão para realizar essa ação.")]
    Unauthorized,
    #[msg("Saldo insuficiente.")]
    InsufficientBalance,
    #[msg("Tokens insuficientes para liberar.")]
    InsufficientTokens,
    #[msg("Nenhuma liberação restante.")]
    NoReleasesRemaining,
    #[msg("Ainda não chegou o momento para liberar tokens.")]
    ReleaseNotDue,
    #[msg("Transferência acima do limite permitido.")]
    WhaleTransferRestricted,
    #[msg("Liberação inicial inválida.")]
    InvalidInitialRelease,
}

// Função auxiliar para calcular a comissão
fn calculate_commission(amount: u64, rate: f64) -> u64 {
    (amount as f64 * rate) as u64
}
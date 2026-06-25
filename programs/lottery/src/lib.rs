use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub(crate) mod switchboard;

pub use error::ErrorCode;
pub use state::TokenLottery;

// Re-export account structs so they're in scope via `use super::*` inside #[program]
pub use instructions::{BuyTicket, ClaimWinnings, CommitRandomness, Initialize, InitializeLottery, RevealWinner};

// The #[program] macro generates `pub use crate::__client_accounts_<Struct>::*`
// so each __client_accounts_* module must exist at the crate root.
pub(crate) use instructions::buy_ticket::__client_accounts_buy_ticket;
pub(crate) use instructions::claim_winnings::__client_accounts_claim_winnings;
pub(crate) use instructions::commit_randomness::__client_accounts_commit_randomness;
pub(crate) use instructions::initialize_config::__client_accounts_initialize;
pub(crate) use instructions::initialize_lottery::__client_accounts_initialize_lottery;
pub(crate) use instructions::reveal_winner::__client_accounts_reveal_winner;

declare_id!("xEdyqr3GsKTevvWHzef1AznGgT9x23ft6aFiNag1ksS");

#[program]
pub mod lottery {
    use super::*;

    pub fn initialize_config(ctx: Context<Initialize>, start: u64, end: u64, price: u64) -> Result<()> {
        instructions::initialize_config::handler(ctx, start, end, price)
    }

    pub fn initialize_lottery(ctx: Context<InitializeLottery>) -> Result<()> {
        instructions::initialize_lottery::handler(ctx)
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        instructions::buy_ticket::handler(ctx)
    }

    pub fn commit_randomness(ctx: Context<CommitRandomness>) -> Result<()> {
        instructions::commit_randomness::handler(ctx)
    }

    pub fn reveal_winner(ctx: Context<RevealWinner>) -> Result<()> {
        instructions::reveal_winner::handler(ctx)
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        instructions::claim_winnings::handler(ctx)
    }
}

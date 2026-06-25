use crate::error::ErrorCode;
use crate::state::TokenLottery;
use crate::switchboard::RandomnessAccountData;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CommitRandomness<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    pub randomness_account: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CommitRandomness>) -> Result<()> {
    if ctx.accounts.caller.key() != ctx.accounts.token_lottery.authority {
        return Err(ErrorCode::NotAuthorized.into());
    }

    let clock = Clock::get()?;
    let randomness =
        RandomnessAccountData::parse(ctx.accounts.randomness_account.data.borrow()).unwrap();

    if randomness.seed_slot != clock.slot - 1 {
        return Err(ErrorCode::RandomnessAlreadyRevealed.into());
    }

    ctx.accounts.token_lottery.randomness_account =
        ctx.accounts.randomness_account.to_account_info().key();

    Ok(())
}

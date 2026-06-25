use crate::error::ErrorCode;
use crate::state::TokenLottery;
use crate::switchboard::RandomnessAccountData;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct RevealWinner<'info> {
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

pub fn handler(ctx: Context<RevealWinner>) -> Result<()> {
    let clock = Clock::get()?;

    if ctx.accounts.caller.key() != ctx.accounts.token_lottery.authority {
        return Err(ErrorCode::NotAuthorized.into());
    }

    if ctx.accounts.randomness_account.key() != ctx.accounts.token_lottery.randomness_account {
        return Err(ErrorCode::IncorrectRandomnessAccount.into());
    }

    if clock.slot < ctx.accounts.token_lottery.start_time {
        return Err(ErrorCode::LotteryNotOpen.into());
    }

    if clock.slot <= ctx.accounts.token_lottery.end_time {
        return Err(ErrorCode::LotteryNotCompleted.into());
    }

    require!(
        ctx.accounts.token_lottery.winner_chosen == false,
        ErrorCode::WinnerChosen
    );

    let randomness_data =
        RandomnessAccountData::parse(ctx.accounts.randomness_account.data.borrow()).unwrap();

    let random_value = randomness_data
        .get_value(&clock)
        .map_err(|_| ErrorCode::RandomnessNotResolve)?;

    let winner = random_value[0] as u64 % ctx.accounts.token_lottery.total_tickets;

    ctx.accounts.token_lottery.winner = winner;
    ctx.accounts.token_lottery.winner_chosen = true;

    Ok(())
}

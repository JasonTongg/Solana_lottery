use anchor_lang::prelude::*;
use crate::error::ErrorCode;
use crate::state::TokenLottery;
use crate::switchboard::RandomnessAccountData;

#[derive(Accounts)]
pub struct RevealWinner<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump = token_lottery.bump
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    /// CHECK: this account is checked by the Switchboard smart contract
    pub randomness_account: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<RevealWinner>) -> Result<()> {
    let clock = Clock::get()?;
    let token_lottery = &mut ctx.accounts.token_lottery;

    if ctx.accounts.payer.key() != token_lottery.authority {
        return Err(ErrorCode::NotAuthorized.into());
    }

    if ctx.accounts.randomness_account.key() != token_lottery.randomness_account {
        return Err(ErrorCode::IncorrectRandomnessAccount.into());
    }

    if clock.slot < token_lottery.end_time {
        return Err(ErrorCode::LotteryNotCompleted.into());
    }

    require!(!token_lottery.winner_chosen, ErrorCode::WinnerChosen);

    let randomness_data =
        RandomnessAccountData::parse(ctx.accounts.randomness_account.data.borrow()).unwrap();

    let reveal_random_value = randomness_data
        .get_value(&clock)
        .map_err(|_| ErrorCode::RandomnessNotResolve)?;

    let winner = reveal_random_value[0] as u64 % token_lottery.total_tickets;

    token_lottery.winner = winner;
    token_lottery.winner_chosen = true;
    Ok(())
}

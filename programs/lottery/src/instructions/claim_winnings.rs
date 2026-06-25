use crate::constants::NAME;
use crate::error::ErrorCode;
use crate::state::TokenLottery;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{Metadata, MetadataAccount},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), ticket_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program
    )]
    pub ticket_metadata: Account<'info, MetadataAccount>,

    #[account(
        mut,
        seeds = [token_lottery.winner.to_le_bytes().as_ref()],
        bump
    )]
    pub ticket_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [b"collection_mint".as_ref()],
        bump
    )]
    pub collection_mint: InterfaceAccount<'info, Mint>,

    #[account(
        associated_token::mint = ticket_mint,
        associated_token::authority = caller,
        associated_token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<ClaimWinnings>) -> Result<()> {
    require!(
        ctx.accounts.token_lottery.winner_chosen == true,
        ErrorCode::WinnerNotChosen
    );
    require!(
        ctx.accounts
            .ticket_metadata
            .collection
            .as_ref()
            .unwrap()
            .verified,
        ErrorCode::NotVerified
    );
    require!(
        ctx.accounts
            .ticket_metadata
            .collection
            .as_ref()
            .unwrap()
            .key
            == ctx.accounts.collection_mint.key(),
        ErrorCode::IncorrectTicket
    );

    let name = NAME.to_owned() + ctx.accounts.token_lottery.winner.to_string().as_str();
    let ticket_name = ctx.accounts.ticket_metadata.name.replace("\u{0}", "");

    require!(name == ticket_name, ErrorCode::IncorrectTicket);
    require!(
        ctx.accounts.user_token_account.amount > 0,
        ErrorCode::NoTicket
    );

    **ctx
        .accounts
        .token_lottery
        .to_account_info()
        .lamports
        .borrow_mut() -= ctx.accounts.token_lottery.lottery_pot_amount;
    **ctx.accounts.caller.to_account_info().lamports.borrow_mut() +=
        ctx.accounts.token_lottery.lottery_pot_amount;

    ctx.accounts.token_lottery.lottery_pot_amount = 0;
    Ok(())
}

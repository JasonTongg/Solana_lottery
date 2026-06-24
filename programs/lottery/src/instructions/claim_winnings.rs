use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{Metadata, MetadataAccount},
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use crate::constants::NAME;
use crate::error::ErrorCode;
use crate::state::TokenLottery;

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump = token_lottery.bump
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    #[account(
        seeds = [token_lottery.winner.to_le_bytes().as_ref()],
        bump
    )]
    pub ticket_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [b"collection_mint".as_ref()],
        bump
    )]
    pub collection_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [b"metadata", token_metadata_program.key().as_ref(), ticket_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program
    )]
    pub ticket_metadata: Account<'info, MetadataAccount>,

    #[account(
        associated_token::mint = ticket_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub ticket_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [b"metadata", token_metadata_program.key().as_ref(), collection_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program
    )]
    pub collection_metadata: Account<'info, MetadataAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,
}

pub fn handler(ctx: Context<ClaimWinnings>) -> Result<()> {
    require!(ctx.accounts.token_lottery.winner_chosen, ErrorCode::WinnerNotChosen);
    require!(
        ctx.accounts.ticket_metadata.collection.as_ref().unwrap().verified,
        ErrorCode::NotVerified
    );
    require!(
        ctx.accounts.ticket_metadata.collection.as_ref().unwrap().key
            == ctx.accounts.collection_mint.key(),
        ErrorCode::IncorrectTicket
    );

    let ticket_name = NAME.to_owned() + &ctx.accounts.token_lottery.winner.to_string();
    let metadata_name = ctx.accounts.ticket_metadata.name.replace("\u{0}", "");

    require!(metadata_name == ticket_name, ErrorCode::IncorrectTicket);
    require!(ctx.accounts.ticket_account.amount > 0, ErrorCode::NoTicket);

    **ctx.accounts.token_lottery.to_account_info().lamports.borrow_mut() -=
        ctx.accounts.token_lottery.lottery_pot_amount;
    **ctx.accounts.payer.to_account_info().lamports.borrow_mut() +=
        ctx.accounts.token_lottery.lottery_pot_amount;

    ctx.accounts.token_lottery.lottery_pot_amount = 0;

    Ok(())
}

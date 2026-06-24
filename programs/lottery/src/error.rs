use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Lottery is not open")]
    LotteryNotOpen,
    #[msg("Not Authorized")]
    NotAuthorized,
    #[msg("Randomness Already Revealed")]
    RandomnessAlreadyRevealed,
    #[msg("Incorrect Randomness Account")]
    IncorrectRandomnessAccount,
    #[msg("Lottery Not Completed")]
    LotteryNotCompleted,
    #[msg("Winner Already Chosen")]
    WinnerChosen,
    #[msg("Randomness Not Resolved")]
    RandomnessNotResolve,
    #[msg("Winner Not Yet Chosen")]
    WinnerNotChosen,
    #[msg("Not Verified")]
    NotVerified,
    #[msg("Incorrect Ticket")]
    IncorrectTicket,
    #[msg("No Ticket Found")]
    NoTicket,
}

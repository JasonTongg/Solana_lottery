// LiteSVM integration tests for the Solana Lottery program.
//
// Run after building the program:
//   anchor build
//   cargo test --manifest-path programs/lottery/Cargo.toml --test test_lottery
//
// Tests marked #[ignore] require the Metaplex Token Metadata program binary.

use anchor_lang::{prelude::Pubkey, AccountDeserialize, AccountSerialize, InstructionData};
use anchor_lang::solana_program::{instruction::{AccountMeta, Instruction}, system_program};
use litesvm::LiteSVM;
use litesvm::types::{FailedTransactionMetadata, TransactionMetadata};
use lottery::{TokenLottery, ID as PROGRAM_ID};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

// ── Constants ────────────────────────────────────────────────────────────────

const PROGRAM_BYTES: &[u8] = include_bytes!("../../../target/deploy/lottery.so");

// sha256("account:RandomnessAccountData")[..8]  — Switchboard discriminator
const RANDOMNESS_DISC: [u8; 8] = [10, 66, 229, 135, 220, 239, 217, 114];

// ── PDAs ─────────────────────────────────────────────────────────────────────

fn lottery_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"token_lottery"], &PROGRAM_ID)
}

fn collection_mint_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"collection_mint"], &PROGRAM_ID)
}

// ── SVM helpers ──────────────────────────────────────────────────────────────

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program(PROGRAM_ID, PROGRAM_BYTES)
        .expect("failed to load lottery.so – run `anchor build` first");
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    ix: Instruction,
    extra: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let bh = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &bh);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx)
}

fn send_ok(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction, extra: &[&Keypair]) {
    send(svm, payer, ix, extra).unwrap_or_else(|e| {
        panic!("expected success, got: {:?}\nlogs: {:#?}", e.err, e.meta.logs)
    });
}

fn send_err(
    svm: &mut LiteSVM,
    payer: &Keypair,
    ix: Instruction,
    extra: &[&Keypair],
) -> FailedTransactionMetadata {
    send(svm, payer, ix, extra).expect_err("expected transaction to fail")
}

fn logs_contain(meta: &FailedTransactionMetadata, needle: &str) -> bool {
    meta.meta.logs.iter().any(|l| l.contains(needle))
}

// ── Account helpers ───────────────────────────────────────────────────────────

fn read_lottery(svm: &LiteSVM) -> TokenLottery {
    let (pda, _) = lottery_pda();
    let acc = svm.get_account(&pda).expect("token_lottery account not found");
    TokenLottery::try_deserialize(&mut acc.data.as_ref())
        .expect("failed to deserialize TokenLottery")
}

/// Deserialize the TokenLottery account, apply `f`, reserialize, and write it back.
fn patch_lottery<F: FnOnce(&mut TokenLottery)>(svm: &mut LiteSVM, f: F) {
    let (pda, _) = lottery_pda();
    let mut acc = svm.get_account(&pda).expect("token_lottery not found");
    let mut lottery = TokenLottery::try_deserialize(&mut acc.data.as_ref()).unwrap();
    f(&mut lottery);
    let mut new_data = Vec::new();
    lottery.try_serialize(&mut new_data).unwrap();
    acc.data = new_data;
    svm.set_account(pda, acc).unwrap();
}

/// Build a fake Switchboard RandomnessAccountData account and inject it into the SVM.
///
/// Byte layout (from `switchboard::RandomnessAccountData::parse`):
/// ```
/// [0..8]    discriminator
/// [8..40]   authority   (Pubkey, zeroed)
/// [40..72]  queue       (Pubkey, zeroed)
/// [72..104] seed_slothash ([u8;32], zeroed)
/// [104..112] seed_slot  (u64 LE)
/// [112..144] oracle     (Pubkey, zeroed)
/// [144..152] reveal_slot (u64 LE)
/// [152..184] value      ([u8;32])
/// ```
fn inject_randomness(svm: &mut LiteSVM, key: Pubkey, seed_slot: u64, reveal_slot: u64, value: [u8; 32]) {
    let mut data = vec![0u8; 184];
    data[0..8].copy_from_slice(&RANDOMNESS_DISC);
    data[104..112].copy_from_slice(&seed_slot.to_le_bytes());
    data[144..152].copy_from_slice(&reveal_slot.to_le_bytes());
    data[152..184].copy_from_slice(&value);

    let acc = svm.get_account(&key).unwrap_or(Account {
        lamports: 1_000_000,
        data: vec![0u8; 184],
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    });
    svm.set_account(key, Account { data, ..acc }).unwrap();
}

// ── Instruction builders ──────────────────────────────────────────────────────

fn ix_initialize_config(payer: Pubkey, start: u64, end: u64, price: u64) -> Instruction {
    let (pda, _) = lottery_pda();
    Instruction::new_with_bytes(
        PROGRAM_ID,
        &lottery::instruction::InitializeConfig { start, end, price }.data(),
        vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}

fn ix_commit_randomness(payer: Pubkey, randomness_key: Pubkey) -> Instruction {
    let (pda, _) = lottery_pda();
    Instruction::new_with_bytes(
        PROGRAM_ID,
        &lottery::instruction::CommitRandomness {}.data(),
        vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(randomness_key, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}

fn ix_reveal_winner(payer: Pubkey, randomness_key: Pubkey) -> Instruction {
    let (pda, _) = lottery_pda();
    Instruction::new_with_bytes(
        PROGRAM_ID,
        &lottery::instruction::RevealWinner {}.data(),
        vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(randomness_key, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: initialize_config
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_config_stores_fields() {
    let (mut svm, payer) = setup();

    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 100, 5_000, 500_000_000), &[]);

    let lottery = read_lottery(&svm);
    assert_eq!(lottery.start_time, 100);
    assert_eq!(lottery.end_time, 5_000);
    assert_eq!(lottery.ticket_price, 500_000_000);
    assert_eq!(lottery.authority, payer.pubkey());
    assert_eq!(lottery.total_tickets, 0);
    assert_eq!(lottery.lottery_pot_amount, 0);
    assert!(!lottery.winner_chosen);
    assert_eq!(lottery.winner, 0);
    assert_eq!(lottery.randomness_account, Pubkey::default());
}

#[test]
fn test_initialize_config_different_values() {
    let (mut svm, payer) = setup();

    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 200, 10_000, 1_000_000_000), &[]);

    let lottery = read_lottery(&svm);
    assert_eq!(lottery.start_time, 200);
    assert_eq!(lottery.end_time, 10_000);
    assert_eq!(lottery.ticket_price, 1_000_000_000);
}

#[test]
fn test_initialize_config_creates_pda() {
    let (mut svm, payer) = setup();
    let (pda, _) = lottery_pda();

    assert!(svm.get_account(&pda).is_none(), "PDA should not exist before init");
    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 0, 1_000, 100_000), &[]);
    assert!(svm.get_account(&pda).is_some(), "PDA should exist after init");
}

#[test]
fn test_initialize_config_cannot_be_called_twice() {
    let (mut svm, payer) = setup();

    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 0, 1_000, 100_000), &[]);

    // Second call must fail — PDA already initialised
    let err = send_err(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 0, 2_000, 200_000), &[]);
    assert!(!err.meta.logs.is_empty(), "expected non-empty logs on failure");
}

#[test]
fn test_initialize_config_authority_is_payer() {
    let (mut svm, authority) = setup();

    send_ok(&mut svm, &authority, ix_initialize_config(authority.pubkey(), 0, 1_000, 100_000), &[]);

    assert_eq!(read_lottery(&svm).authority, authority.pubkey());
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: commit_randomness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_commit_randomness_success() {
    let (mut svm, payer) = setup();
    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 50, 5_000, 100_000), &[]);

    // At slot 100 the randomness account must have seed_slot = 99 (clock.slot - 1)
    svm.warp_to_slot(100);
    let randomness_kp = Keypair::new();
    inject_randomness(&mut svm, randomness_kp.pubkey(), 99, 500, [7u8; 32]);

    send_ok(&mut svm, &payer, ix_commit_randomness(payer.pubkey(), randomness_kp.pubkey()), &[]);

    assert_eq!(read_lottery(&svm).randomness_account, randomness_kp.pubkey());
}

#[test]
fn test_commit_randomness_stores_account_key() {
    let (mut svm, payer) = setup();
    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 0, 9_999, 100_000), &[]);

    svm.warp_to_slot(200);
    let rnd_kp = Keypair::new();
    inject_randomness(&mut svm, rnd_kp.pubkey(), 199, 800, [42u8; 32]);
    send_ok(&mut svm, &payer, ix_commit_randomness(payer.pubkey(), rnd_kp.pubkey()), &[]);

    assert_eq!(read_lottery(&svm).randomness_account, rnd_kp.pubkey());
}

#[test]
fn test_commit_randomness_wrong_authority() {
    let (mut svm, payer) = setup();
    let intruder = Keypair::new();
    svm.airdrop(&intruder.pubkey(), 1_000_000_000).unwrap();

    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 50, 5_000, 100_000), &[]);

    svm.warp_to_slot(100);
    let rnd_kp = Keypair::new();
    inject_randomness(&mut svm, rnd_kp.pubkey(), 99, 500, [1u8; 32]);

    let err = send_err(&mut svm, &intruder, ix_commit_randomness(intruder.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(logs_contain(&err, "NotAuthorized"), "logs: {:#?}", err.meta.logs);
}

#[test]
fn test_commit_randomness_stale_seed_slot() {
    let (mut svm, payer) = setup();
    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 50, 5_000, 100_000), &[]);

    svm.warp_to_slot(100);
    let rnd_kp = Keypair::new();
    // seed_slot=50 but we need 99 → RandomnessAlreadyRevealed
    inject_randomness(&mut svm, rnd_kp.pubkey(), 50, 500, [1u8; 32]);

    let err = send_err(&mut svm, &payer, ix_commit_randomness(payer.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(logs_contain(&err, "RandomnessAlreadyRevealed"), "logs: {:#?}", err.meta.logs);
}

#[test]
fn test_commit_randomness_invalid_discriminator() {
    let (mut svm, payer) = setup();
    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 50, 5_000, 100_000), &[]);

    svm.warp_to_slot(100);

    // Inject account with all-zero data (wrong discriminator)
    let rnd_kp = Keypair::new();
    svm.set_account(rnd_kp.pubkey(), Account {
        lamports: 1_000_000,
        data: vec![0u8; 184],
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    }).unwrap();

    let result = send(&mut svm, &payer, ix_commit_randomness(payer.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(result.is_err(), "should fail with invalid randomness discriminator");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: reveal_winner
// ─────────────────────────────────────────────────────────────────────────────

/// Full setup helper: init config → patch total_tickets → commit at slot 100 → return randomness kp.
/// The reveal should happen at slot 250 (= reveal_slot, AND > end_time=200).
fn setup_for_reveal(
    svm: &mut LiteSVM,
    payer: &Keypair,
    total_tickets: u64,
    value: [u8; 32],
) -> Keypair {
    send_ok(svm, payer, ix_initialize_config(payer.pubkey(), 10, 200, 100_000), &[]);

    // Inject total_tickets so reveal_winner doesn't divide-by-zero
    patch_lottery(svm, |l| l.total_tickets = total_tickets);

    svm.warp_to_slot(100);
    let rnd_kp = Keypair::new();
    inject_randomness(svm, rnd_kp.pubkey(), 99, 250, value);
    send_ok(svm, payer, ix_commit_randomness(payer.pubkey(), rnd_kp.pubkey()), &[]);

    rnd_kp
}

#[test]
fn test_reveal_winner_success() {
    let (mut svm, payer) = setup();

    // value[0]=10, total_tickets=5 → winner = 10 % 5 = 0
    let mut value = [0u8; 32];
    value[0] = 10;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    svm.warp_to_slot(250); // == reveal_slot AND > end_time
    send_ok(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), rnd_kp.pubkey()), &[]);

    let lottery = read_lottery(&svm);
    assert!(lottery.winner_chosen);
    assert_eq!(lottery.winner, 0); // 10 % 5 == 0
}

#[test]
fn test_reveal_winner_modulo_index() {
    let (mut svm, payer) = setup();

    // value[0]=7, total_tickets=3 → winner = 7 % 3 = 1
    let mut value = [0u8; 32];
    value[0] = 7;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 3, value);

    svm.warp_to_slot(250);
    send_ok(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), rnd_kp.pubkey()), &[]);

    let lottery = read_lottery(&svm);
    assert!(lottery.winner_chosen);
    assert_eq!(lottery.winner, 1); // 7 % 3 == 1
}

#[test]
fn test_reveal_winner_sets_winner_chosen_flag() {
    let (mut svm, payer) = setup();

    let mut value = [0u8; 32];
    value[0] = 5;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    assert!(!read_lottery(&svm).winner_chosen, "should not be chosen before reveal");

    svm.warp_to_slot(250);
    send_ok(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), rnd_kp.pubkey()), &[]);

    assert!(read_lottery(&svm).winner_chosen, "should be chosen after reveal");
}

#[test]
fn test_reveal_winner_not_authorized() {
    let (mut svm, payer) = setup();
    let intruder = Keypair::new();
    svm.airdrop(&intruder.pubkey(), 1_000_000_000).unwrap();

    let mut value = [0u8; 32];
    value[0] = 5;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    svm.warp_to_slot(250);
    let err = send_err(&mut svm, &intruder, ix_reveal_winner(intruder.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(logs_contain(&err, "NotAuthorized"), "logs: {:#?}", err.meta.logs);
}

#[test]
fn test_reveal_winner_lottery_not_completed() {
    let (mut svm, payer) = setup();

    let mut value = [0u8; 32];
    value[0] = 3;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    // Stay at slot 150 – before end_time=200
    svm.warp_to_slot(150);
    let err = send_err(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(logs_contain(&err, "LotteryNotCompleted"), "logs: {:#?}", err.meta.logs);
}

#[test]
fn test_reveal_winner_incorrect_randomness_account() {
    let (mut svm, payer) = setup();

    let mut value = [0u8; 32];
    value[0] = 3;
    let _committed_rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    // Pass a different (wrong) randomness key
    let wrong_kp = Keypair::new();
    inject_randomness(&mut svm, wrong_kp.pubkey(), 99, 250, value);

    svm.warp_to_slot(250);
    let err = send_err(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), wrong_kp.pubkey()), &[]);
    assert!(logs_contain(&err, "IncorrectRandomnessAccount"), "logs: {:#?}", err.meta.logs);
}

#[test]
fn test_reveal_winner_randomness_not_resolved() {
    let (mut svm, payer) = setup();

    let mut value = [0u8; 32];
    value[0] = 3;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    // Warp past end_time but NOT to reveal_slot=250 → get_value check fails
    svm.warp_to_slot(220);
    let err = send_err(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(
        logs_contain(&err, "RandomnessNotResolve") || logs_contain(&err, "Custom(6006)"),
        "logs: {:#?}", err.meta.logs
    );
}

#[test]
fn test_reveal_winner_cannot_reveal_twice() {
    let (mut svm, payer) = setup();

    let mut value = [0u8; 32];
    value[0] = 5;
    let rnd_kp = setup_for_reveal(&mut svm, &payer, 5, value);

    // Directly inject winner_chosen=true so we get a fresh blockhash on the reveal call.
    // (Calling reveal_winner twice with the same instruction data reuses the same tx signature
    //  which is rejected at transport level before reaching the program.)
    patch_lottery(&mut svm, |l| {
        l.winner_chosen = true;
        l.winner = 0;
    });

    svm.warp_to_slot(250); // past end_time=200 AND equals reveal_slot
    let err = send_err(&mut svm, &payer, ix_reveal_winner(payer.pubkey(), rnd_kp.pubkey()), &[]);
    assert!(logs_contain(&err, "WinnerChosen"), "logs: {:#?}", err.meta.logs);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: initialize_lottery, buy_ticket, claim_winnings
// (require the Metaplex Token Metadata program binary – ignored by default)
// ─────────────────────────────────────────────────────────────────────────────
//
// To enable:
//  1. Download mpl_token_metadata.so  (solana program dump metaqbxx... mpl.so)
//  2. Add the bytes: const METADATA_BYTES: &[u8] = include_bytes!("path/to/mpl.so");
//  3. Load it: svm.add_program(metaplex_program_id(), METADATA_BYTES).unwrap();
//  4. Remove the #[ignore] attribute and fill in the full account-meta lists.

fn metaplex_program_id() -> Pubkey {
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s".parse().unwrap()
}

#[test]
#[ignore = "requires mpl_token_metadata.so – see comment above"]
fn test_initialize_lottery_creates_collection_nft() {
    let (mut svm, payer) = setup();
    // svm.add_program(metaplex_program_id(), METADATA_BYTES).unwrap();
    let _ = metaplex_program_id();

    send_ok(&mut svm, &payer, ix_initialize_config(payer.pubkey(), 0, 10_000, 100_000), &[]);

    let (collection_mint, _) = collection_mint_pda();

    // Build initialize_lottery with the full account-meta list once the binary is available.
    let _init_lottery_ix = Instruction::new_with_bytes(
        PROGRAM_ID,
        &lottery::instruction::InitializeLottery {}.data(),
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(collection_mint, false),
            // collection_token_account, metadata, master_edition,
            // token_program, system_program, associated_token_program,
            // token_metadata_program, rent
        ],
    );
    // send_ok(&mut svm, &payer, _init_lottery_ix, &[]);
    // assert!(svm.get_account(&collection_mint).is_some());
    todo!("fill in full account-meta list and uncomment assertions")
}

#[test]
#[ignore = "requires mpl_token_metadata.so and an initialised collection"]
fn test_buy_ticket_increments_counter_and_charges_fee() {
    let (mut _svm, _payer) = setup();
    // Full flow: initialize_config → initialize_lottery → buy_ticket
    // Assertions: total_tickets += 1, lottery_pot_amount += ticket_price
    todo!("implement after Metaplex binary is available")
}

#[test]
#[ignore = "requires mpl_token_metadata.so and a completed lottery"]
fn test_claim_winnings_transfers_pot_to_winner() {
    let (mut _svm, _payer) = setup();
    // Full flow: initialize_config → initialize_lottery → buy_ticket(s)
    //            → commit_randomness → reveal_winner → claim_winnings
    // Assertions: lottery_pot_amount == 0, winner lamports increased
    todo!("implement after Metaplex binary is available")
}

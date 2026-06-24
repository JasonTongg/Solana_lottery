use std::cell::Ref;
use anchor_lang::prelude::{Clock, ProgramError};

// Discriminator: sha256("account:RandomnessAccountData")[..8]
const DISCRIMINATOR: [u8; 8] = [10, 66, 229, 135, 220, 239, 217, 114];

pub struct RandomnessAccountData {
    pub seed_slot: u64,
    pub reveal_slot: u64,
    pub value: [u8; 32],
}

impl RandomnessAccountData {
    // Byte offsets (including the 8-byte discriminator prefix):
    // [8..40]    authority     (Pubkey)
    // [40..72]   queue         (Pubkey)
    // [72..104]  seed_slothash ([u8;32])
    // [104..112] seed_slot     (u64 LE)
    // [112..144] oracle        (Pubkey)
    // [144..152] reveal_slot   (u64 LE)
    // [152..184] value         ([u8;32])
    pub fn parse(data: Ref<&mut [u8]>) -> Result<Self, ProgramError> {
        if data.len() < 184 {
            return Err(ProgramError::InvalidAccountData);
        }
        let mut disc = [0u8; 8];
        disc.copy_from_slice(&data[..8]);
        if disc != DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        let seed_slot = u64::from_le_bytes(data[104..112].try_into().unwrap());
        let reveal_slot = u64::from_le_bytes(data[144..152].try_into().unwrap());
        let mut value = [0u8; 32];
        value.copy_from_slice(&data[152..184]);
        Ok(Self { seed_slot, reveal_slot, value })
    }

    pub fn get_value(&self, clock: &Clock) -> Result<[u8; 32], ProgramError> {
        if clock.slot != self.reveal_slot {
            return Err(ProgramError::Custom(6000));
        }
        Ok(self.value)
    }
}

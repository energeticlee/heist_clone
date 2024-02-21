use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Update Authority")]
    InvalidUpdateAuthority,
    #[msg("Invalid Game Master Mint Address")]
    InvalidMintAddress,
    #[msg("Invalid NFT not part of Collection")]
    MismatchCollection,
}

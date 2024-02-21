use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::*};

use crate::state::*;

#[derive(Accounts)]
pub struct InitOrUpdateCollection<'info> {
    #[account(init_if_needed, seeds=[b"global"], bump, payer = creator, space= Global::len())]
    pub global_state: Account<'info, Global>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        constraint = user_nft_account.owner.key() == creator.key(),
        constraint = user_nft_account.amount == 1,
        constraint = user_nft_account.mint == user_nft_mint.key(),
    )]
    pub user_nft_account: Account<'info, TokenAccount>,
    // metadata required to check for collection verification
    /// CHECK: Account will be validated in processor
    pub nft_metadata: AccountInfo<'info>,
    pub user_nft_mint: Account<'info, Mint>,
    pub collection_mint: Account<'info, Mint>,
    #[account(
        constraint = global_reward_token_account.mint == reward_mint.key(),
        constraint = global_reward_token_account.owner == creator.key()
    )]
    pub creator_token_reward_account: Account<'info, TokenAccount>,
    pub reward_mint: Account<'info, Mint>,
    #[account(
        constraint = global_reward_token_account.mint == reward_mint.key(),
        constraint = global_reward_token_account.owner == global_state.key()
    )]
    pub global_reward_token_account: Account<'info, TokenAccount>,
    // ATA Program required to create ATA for pda_nft_account
    // Token Program required to call transfer instruction
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateCollectionAuthority<'info> {
    #[account(mut, seeds=[b"global"], bump)]
    pub global_state: Account<'info, Global>,
    #[account(
        mut,
        constraint = global_state.update_authority == current_authority.key()
    )]
    pub current_authority: Signer<'info>,
    #[account(mut)]
    /// CHECK:
    pub new_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakePlayerStakeInfo<'info> {
    #[account(mut, seeds = [b"global"], bump)]
    pub global_state: Account<'info, Global>,
    #[account(init_if_needed, seeds = [b"player", player.key().as_ref()], bump, payer = player, space = PlayerInfo::len())]
    pub player_info: Account<'info, PlayerInfo>,
    #[account(init, seeds = [b"stake_info", player.key().as_ref(), user_nft_account.key().as_ref()], bump, payer = player, space = PlayerStakeInfo::len())]
    pub player_stake_info: Account<'info, PlayerStakeInfo>,
    #[account(
        mut,
        constraint = user_nft_account.owner.key() == player.key(),
        constraint = user_nft_account.amount == 1,
        constraint = user_nft_account.mint == user_nft_mint.key(),
    )]
    pub user_nft_account: Account<'info, TokenAccount>,
    // metadata required to check for collection verification
    /// CHECK: Account will be validated in processor
    pub nft_metadata: AccountInfo<'info>,
    pub user_nft_mint: Account<'info, Mint>,
    #[account(constraint = collection_mint.key() == global_state.collection)]
    pub collection_mint: Account<'info, Mint>,
    #[account(mut)]
    pub player: Signer<'info>,
    // Token Program required to call delegate instruction
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UnstakePlayerStakeInfo<'info> {
    #[account(mut, seeds = [b"global"], bump)]
    pub global_state: Account<'info, Global>,
    #[account(
        mut, seeds = [b"player", player.key().as_ref()], bump,
        constraint = player_info.active_staked >= 1
    )]
    pub player_info: Account<'info, PlayerInfo>,
    #[account(
        mut, seeds = [b"stake_info", player.key().as_ref(), nft_mint.key().as_ref()], bump,
        constraint = player_stake_info.mint == nft_mint.key(),
        constraint = player_stake_info.owner == player.key(),
        close = player
    )]
    pub player_stake_info: Account<'info, PlayerStakeInfo>,
    #[account(
        init_if_needed,
        payer = player, // If init required, payer will be initializer
        associated_token::mint = reward_mint, // If init required, mint will be set to Mint
        associated_token::authority = player // If init required, authority set to PDA
    )]
    pub player_reward_token_account: Account<'info, TokenAccount>,
    #[account(constraint = global_state.reward_mint == reward_mint.key())]
    pub reward_mint: Account<'info, Mint>,
    #[account(
        constraint = global_reward_token_account.mint == reward_mint.key(),
        constraint = global_reward_token_account.owner == global_state.key()
    )]
    pub global_reward_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        constraint = user_nft_account.owner.key() == player.key(),
        constraint = user_nft_account.amount == 1,
        constraint = user_nft_account.mint == nft_mint.key(),
    )]
    pub user_nft_account: Account<'info, TokenAccount>,
    // metadata required to check for collection verification
    /// CHECK: Account will be validated in processor
    pub nft_metadata: AccountInfo<'info>,
    pub nft_mint: Account<'info, Mint>,
    #[account(constraint = collection_mint.key() == global_state.collection)]
    pub collection_mint: Account<'info, Mint>,
    // ATA Program required to create ATA for pda_nft_account
    pub associated_token_program: Program<'info, AssociatedToken>,
    // Token Program required to call revoke instruction
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

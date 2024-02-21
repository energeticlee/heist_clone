use crate::{error::ErrorCode, state::Role, InitOrUpdateCollection};
use crate::{state::*, StakePlayerStakeInfo, UnstakePlayerStakeInfo, UpdateCollectionAuthority};
use anchor_lang::prelude::*;
use anchor_spl::token::{approve, revoke, transfer, Approve, Revoke, Transfer};
use mpl_token_metadata::state::{Metadata, TokenMetadataAccount};

pub fn init_or_update_collection_processor(
    ctx: Context<InitOrUpdateCollection>,
    end_date: u64,
    reward_per_hour: u64,
) -> Result<()> {
    let metadata: Metadata =
        Metadata::from_account_info(&ctx.accounts.nft_metadata.to_account_info())?;
    let collection = metadata.collection.unwrap();
    if collection.key != ctx.accounts.collection_mint.key() && !collection.verified {
        return err!(ErrorCode::MismatchCollection);
    };
    if metadata.mint != ctx.accounts.user_nft_mint.key() {
        return err!(ErrorCode::InvalidMintAddress);
    }

    let global_state = &mut ctx.accounts.global_state;

    if !global_state.is_initialized {
        global_state.is_initialized = true;
        global_state.total_supply = 0;
        global_state.total_player = 0;
        global_state.collection = ctx.accounts.collection_mint.key();
        global_state.update_authority = ctx.accounts.creator.key();

        global_state.roles = Role::new();
        global_state.banks = Bank::generate_banks(reward_per_hour);
    } else if global_state.update_authority != *ctx.accounts.creator.key {
        return err!(ErrorCode::InvalidUpdateAuthority);
    }

    global_state.end_date = end_date;

    // Calculate Total Reward Required
    let total_reward_per_hour = global_state.banks.len() as u64 * reward_per_hour;
    let time_to_end = global_state.end_date - Clock::get().unwrap().unix_timestamp as u64;
    let total_reward_amount: u64 = time_to_end * total_reward_per_hour;

    // Transfer Reward token to Global State
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Transfer {
        from: ctx.accounts.creator_token_reward_account.to_account_info(),
        to: ctx.accounts.global_reward_token_account.to_account_info(),
        authority: ctx.accounts.creator.to_account_info(),
    };
    let token_transfer_context = CpiContext::new(cpi_program, cpi_accounts);
    transfer(token_transfer_context, total_reward_amount)?;

    global_state.reward_mint = ctx.accounts.reward_mint.key();

    Ok(())
}

pub fn update_collection_authority_processor(
    ctx: Context<UpdateCollectionAuthority>,
) -> Result<()> {
    ctx.accounts.global_state.update_authority = *ctx.accounts.new_authority.key;
    Ok(())
}

pub fn stake_player_stake_info_processor(
    ctx: Context<StakePlayerStakeInfo>,
    bank_tier_risk: BankTierRisk,
) -> Result<()> {
    // Verify if player owns the correct NFT
    let metadata: Metadata =
        Metadata::from_account_info(&ctx.accounts.nft_metadata.to_account_info())?;
    let collection = metadata.collection.unwrap();
    if collection.key != ctx.accounts.collection_mint.key() && !collection.verified {
        return err!(ErrorCode::MismatchCollection);
    };
    if metadata.mint != ctx.accounts.user_nft_mint.key() {
        return err!(ErrorCode::InvalidMintAddress);
    }

    if !ctx.accounts.player_info.is_initialized {
        ctx.accounts.player_info.is_initialized = true;
        ctx.accounts.player_info.point_balance = 0;
        ctx.accounts.player_info.active_staked = 0;

        ctx.accounts.global_state.total_player += 1;
    }

    // Proceed to Delegate
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Approve {
        to: ctx.accounts.user_nft_account.to_account_info(),
        delegate: ctx.accounts.player_stake_info.to_account_info(),
        authority: ctx.accounts.player.to_account_info(),
    };
    let delegate_token_context = CpiContext::new(cpi_program, cpi_accounts);
    approve(delegate_token_context, 1)?;

    // Update player_into
    ctx.accounts.player_info.active_staked += 1;

    // Update player_stake_into
    ctx.accounts.player_stake_info.owner = ctx.accounts.player.key();
    ctx.accounts.player_stake_info.mint = ctx.accounts.user_nft_mint.key();
    ctx.accounts.player_stake_info.bank = bank_tier_risk;
    ctx.accounts.player_stake_info.stake_start_time = Clock::get().unwrap().unix_timestamp as u64;

    // Update global_state
    match bank_tier_risk {
        BankTierRisk::Low => ctx.accounts.global_state.banks[0].total_staked += 1,
        BankTierRisk::Mid => ctx.accounts.global_state.banks[1].total_staked += 1,
        BankTierRisk::High => ctx.accounts.global_state.banks[2].total_staked += 1,
    }

    Ok(())
}

pub fn unstake_player_stake_info_processor(ctx: Context<UnstakePlayerStakeInfo>) -> Result<()> {
    // Verify if unstake NFT input is valid
    let metadata: Metadata =
        Metadata::from_account_info(&ctx.accounts.nft_metadata.to_account_info())?;
    let collection = metadata.collection.unwrap();
    if collection.key != ctx.accounts.collection_mint.key() && !collection.verified {
        return err!(ErrorCode::MismatchCollection);
    };
    if metadata.mint != ctx.accounts.nft_mint.key() {
        return err!(ErrorCode::InvalidMintAddress);
    }

    // Proceed to revoke delegate
    let auth_bump = *ctx.bumps.get("player_stake_info").unwrap();
    let seeds = &[
        b"stake_info".as_ref(),
        &ctx.accounts.player.key().to_bytes(),
        &ctx.accounts.nft_mint.key().to_bytes(),
        &[auth_bump],
    ];
    let signer = &[&seeds[..]];
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Revoke {
        source: ctx.accounts.user_nft_account.to_account_info(),
        authority: ctx.accounts.player.to_account_info(),
    };
    let revoke_token_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    revoke(revoke_token_context)?;

    // CALCULATE REWARDS POINTS AND ADD TO POINT BALANCE
    let player_info = &mut ctx.accounts.player_info;
    let player_stake_info = &mut ctx.accounts.player_stake_info;

    let bank: Bank;
    match player_stake_info.bank {
        BankTierRisk::Low => bank = ctx.accounts.global_state.banks[0].clone(),
        BankTierRisk::Mid => bank = ctx.accounts.global_state.banks[1].clone(),
        BankTierRisk::High => bank = ctx.accounts.global_state.banks[2].clone(),
    }

    let current_time = Clock::get().unwrap().unix_timestamp as u64;
    let base_reward_amount =
        (current_time - player_stake_info.stake_start_time) / 3600 * bank.reward_per_hour;

    let pseudo_random_number = generate_random_number(&ctx.accounts.player.key()); // Between 1 - 100_000

    let multiplier = bank
        .bank_outcomes
        .iter()
        .find(|&&bank_tier| u64::from(bank_tier.odds) > pseudo_random_number)
        .unwrap();

    let reward_amount = base_reward_amount * u64::from(multiplier.payout_multiplier);

    player_info.point_balance += reward_amount;
    player_info.active_staked -= 1;

    // TRANSFER REWARD TOKEN
    let auth_bump = *ctx.bumps.get("global_state").unwrap();
    let seeds = &[b"global".as_ref(), &[auth_bump]];
    let signer = &[&seeds[..]];
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Transfer {
        from: ctx.accounts.global_reward_token_account.to_account_info(),
        to: ctx.accounts.player_reward_token_account.to_account_info(),
        authority: ctx.accounts.global_state.to_account_info(),
    };
    let token_transfer_context = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    transfer(token_transfer_context, reward_amount)?;

    // test: closing a non existing account
    if player_info.active_staked == 0 {
        ctx.accounts.global_state.total_player -= 1;
    }

    Ok(())
}

fn generate_random_number(pubkey: &Pubkey) -> u64 {
    const A: u64 = 1664525;
    const C: u64 = 1013904223;
    const M: u64 = 1 << 32;

    let time = Clock::get().unwrap().unix_timestamp as u64;

    let psudo_random_number = time ^ pubkey.as_ref().iter().fold(0u64, |acc, &x| acc + x as u64);

    let value = (A * psudo_random_number + C) % M;

    (value as f64 / M as f64 * 100_000.0).floor() as u64
}

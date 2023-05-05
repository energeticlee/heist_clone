use anchor_lang::{__private::ZeroCopyAccessor, prelude::*};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{approve, revoke, transfer, Mint, Token, TokenAccount, Transfer},
};
use mpl_token_metadata::state::{Metadata, TokenMetadataAccount};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod heist {
    use super::*;
    use anchor_spl::token::{Approve, Revoke};

    pub fn init_or_update_collection(
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

    pub fn update_collection_authority(ctx: Context<UpdateCollectionAuthority>) -> Result<()> {
        ctx.accounts.global_state.update_authority = *ctx.accounts.new_authority.key;
        Ok(())
    }

    pub fn stake_player_stake_info(
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
        ctx.accounts.player_stake_info.stake_start_time =
            Clock::get().unwrap().unix_timestamp as u64;

        // Update global_state
        match bank_tier_risk {
            BankTierRisk::Low => ctx.accounts.global_state.banks[0].total_staked += 1,
            BankTierRisk::Mid => ctx.accounts.global_state.banks[1].total_staked += 1,
            BankTierRisk::High => ctx.accounts.global_state.banks[2].total_staked += 1,
        }

        Ok(())
    }

    pub fn unstake_player_stake_info(ctx: Context<UnstakePlayerStakeInfo>) -> Result<()> {
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
}

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

#[account]
pub struct Global {
    collection: Pubkey,
    total_supply: u64,
    end_date: u64,
    reward_mint: Pubkey,
    banks: Vec<Bank>,
    roles: Vec<Role>,
    is_initialized: bool,
    update_authority: Pubkey,
    total_player: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, ZeroCopyAccessor)]
pub struct Bank {
    bank_tier: BankTierRisk,
    reward_per_hour: u64,
    total_staked: u64,
    bank_outcomes: Vec<BankOutcome>,
}

impl Bank {
    pub fn generate_banks(reward_per_hour: u64) -> Vec<Bank> {
        vec![
            Bank {
                bank_tier: BankTierRisk::Low,
                reward_per_hour,
                total_staked: 0,
                bank_outcomes: BankOutcome::generate_bank(BankTierRisk::Low),
            },
            Bank {
                bank_tier: BankTierRisk::Mid,
                reward_per_hour,
                total_staked: 0,
                bank_outcomes: BankOutcome::generate_bank(BankTierRisk::Mid),
            },
            Bank {
                bank_tier: BankTierRisk::High,
                reward_per_hour,
                total_staked: 0,
                bank_outcomes: BankOutcome::generate_bank(BankTierRisk::High),
            },
        ]
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct BankOutcome {
    odds: u32,
    payout_multiplier: u8,
    is_negative: bool,
    negative_outcome: NegativeOutcome,
}

impl BankOutcome {
    pub fn new(
        is_negative: bool,
        negative_outcome: NegativeOutcome,
        odds: u32,
        payout_multiplier: u8,
    ) -> BankOutcome {
        BankOutcome {
            is_negative,
            negative_outcome,
            odds, // 40%
            payout_multiplier,
        }
    }

    pub fn generate_bank(risk: BankTierRisk) -> Vec<BankOutcome> {
        match risk {
            BankTierRisk::Low => {
                vec![
                    BankOutcome::new(false, NegativeOutcome::None, 54_000, 1),
                    BankOutcome::new(false, NegativeOutcome::None, 13_000, 2),
                    BankOutcome::new(false, NegativeOutcome::None, 2_000, 5),
                    BankOutcome::new(false, NegativeOutcome::None, 1_000, 10),
                    BankOutcome::new(true, NegativeOutcome::Fumbled, 29_947, 0),
                    BankOutcome::new(true, NegativeOutcome::Confiscation, 45, 0),
                    BankOutcome::new(true, NegativeOutcome::Arrested, 8, 0),
                    BankOutcome::new(true, NegativeOutcome::Rekt, 0, 0),
                ]
            }
            BankTierRisk::Mid => {
                vec![
                    BankOutcome::new(false, NegativeOutcome::None, 45_000, 1),
                    BankOutcome::new(false, NegativeOutcome::None, 10_000, 2),
                    BankOutcome::new(false, NegativeOutcome::None, 3_000, 5),
                    BankOutcome::new(false, NegativeOutcome::None, 2_000, 10),
                    BankOutcome::new(true, NegativeOutcome::Fumbled, 39_924, 0),
                    BankOutcome::new(true, NegativeOutcome::Confiscation, 68, 0),
                    BankOutcome::new(true, NegativeOutcome::Arrested, 8, 0),
                    BankOutcome::new(true, NegativeOutcome::Rekt, 0, 0),
                ]
            }
            BankTierRisk::High => {
                vec![
                    BankOutcome::new(false, NegativeOutcome::None, 36_000, 1),
                    BankOutcome::new(false, NegativeOutcome::None, 7_000, 2),
                    BankOutcome::new(false, NegativeOutcome::None, 4_000, 5),
                    BankOutcome::new(false, NegativeOutcome::None, 3_000, 10),
                    BankOutcome::new(true, NegativeOutcome::Fumbled, 49_897, 0),
                    BankOutcome::new(true, NegativeOutcome::Confiscation, 88, 0),
                    BankOutcome::new(true, NegativeOutcome::Arrested, 14, 0),
                    BankOutcome::new(true, NegativeOutcome::Rekt, 2, 0),
                ]
            }
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum NegativeOutcome {
    None,
    Fumbled,
    Confiscation,
    Arrested,
    Rekt,
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum BankTierRisk {
    Low,
    Mid,
    High,
}

#[account]
pub struct PlayerInfo {
    is_initialized: bool,
    point_balance: u64,
    active_staked: u16,
}

#[account]
pub struct PlayerStakeInfo {
    owner: Pubkey,
    mint: Pubkey,
    bank: BankTierRisk,
    stake_start_time: u64,
    role: Role,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Role {
    role_type: RoleType,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
enum RoleType {
    Chimp,
    Gorrila,
}

impl Role {
    pub fn new() -> Vec<Role> {
        vec![
            Role {
                role_type: RoleType::Chimp,
            },
            Role {
                role_type: RoleType::Gorrila,
            },
        ]
    }
}

const DISCRIMINATOR: usize = 8;
const PUBKEY: usize = 32;
const BOOL: usize = 1;
const U8: usize = 1;
const U16: usize = 2;
const U32: usize = 4;
const U64: usize = 8;
const PREFIX: usize = 4;
const CHAR: usize = 4;

impl Global {
    fn len() -> usize {
        DISCRIMINATOR + PUBKEY + U64 + U64 + PUBKEY + U8 + PUBKEY + U16
    }
}

impl Role {
    fn len(name: &str) -> usize {
        DISCRIMINATOR + BOOL + U8 + PREFIX + (CHAR * name.len())
    }
}

impl Bank {
    fn len(name: &str) -> usize {
        DISCRIMINATOR + U8 + PREFIX + (CHAR * name.len()) + U64 + U64 + U64 + U8 + U64 + BOOL + U32
    }
}

// impl BankTier {
//     fn len() -> usize {
//         DISCRIMINATOR + U32 + U8 + U8 + U8 + U8 + U8 + BOOL
//     }
// }

impl PlayerInfo {
    fn len() -> usize {
        DISCRIMINATOR + BOOL + U64 + U16
    }
}

impl PlayerStakeInfo {
    fn len() -> usize {
        DISCRIMINATOR + PUBKEY + PUBKEY + PUBKEY + U64 + U8 + U8
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Update Authority")]
    InvalidUpdateAuthority,
    #[msg("Invalid Game Master Mint Address")]
    InvalidMintAddress,
    #[msg("Invalid NFT not part of Collection")]
    MismatchCollection,
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

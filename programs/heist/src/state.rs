use anchor_lang::{__private::ZeroCopyAccessor, prelude::*};

#[account]
pub struct Global {
    pub collection: Pubkey,
    pub total_supply: u64,
    pub end_date: u64,
    pub reward_mint: Pubkey,
    pub banks: Vec<Bank>,
    pub roles: Vec<Role>,
    pub is_initialized: bool,
    pub update_authority: Pubkey,
    pub total_player: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, ZeroCopyAccessor)]
pub struct Bank {
    pub bank_tier: BankTierRisk,
    pub reward_per_hour: u64,
    pub total_staked: u64,
    pub bank_outcomes: Vec<BankOutcome>,
}

#[account]
pub struct PlayerInfo {
    pub is_initialized: bool,
    pub point_balance: u64,
    pub active_staked: u16,
}

#[account]
pub struct PlayerStakeInfo {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub bank: BankTierRisk,
    pub stake_start_time: u64,
    pub role: Role,
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
    pub odds: u32,
    pub payout_multiplier: u8,
    pub is_negative: bool,
    pub negative_outcome: NegativeOutcome,
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
const U64: usize = 8;

impl Global {
    pub fn len() -> usize {
        DISCRIMINATOR + PUBKEY + U64 + U64 + PUBKEY + U8 + PUBKEY + U16
    }
}

impl PlayerInfo {
    pub fn len() -> usize {
        DISCRIMINATOR + BOOL + U64 + U16
    }
}

impl PlayerStakeInfo {
    pub fn len() -> usize {
        DISCRIMINATOR + PUBKEY + PUBKEY + PUBKEY + U64 + U8 + U8
    }
}

mod error;
mod processor;
mod state;
mod validator;

use processor::*;
use state::*;
use validator::*;

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod heist {
    use super::*;

    pub fn init_or_update_collection(
        ctx: Context<InitOrUpdateCollection>,
        end_date: u64,
        reward_per_hour: u64,
    ) -> Result<()> {
        init_or_update_collection_processor(ctx, end_date, reward_per_hour)?;
        Ok(())
    }

    pub fn update_collection_authority(ctx: Context<UpdateCollectionAuthority>) -> Result<()> {
        update_collection_authority_processor(ctx)?;
        Ok(())
    }

    pub fn stake_player_stake_info(
        ctx: Context<StakePlayerStakeInfo>,
        bank_tier_risk: BankTierRisk,
    ) -> Result<()> {
        stake_player_stake_info_processor(ctx, bank_tier_risk)?;
        Ok(())
    }

    pub fn unstake_player_stake_info(ctx: Context<UnstakePlayerStakeInfo>) -> Result<()> {
        unstake_player_stake_info_processor(ctx)?;
        Ok(())
    }
}

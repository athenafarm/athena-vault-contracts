use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Controller address
    pub controller: String,
    /// Vault address
    pub vault: String,
    /// Performance fee percentage
    pub performance_fee: Decimal,
    /// underlying denom
    pub stable_denom: String,

    /// Anchor market address to stake UST
    pub anchor_market: String,
    /// Mirror token contract address
    pub mirror_token: String,
    /// Mirror staking contract address
    pub mirror_staking: String,
    /// Mirror mint contract address
    pub mirror_mint: String,
    /// Mirror oracle contract address
    pub mirror_oracle: String,
    /// Terra swap factory contract address
    pub terraswap_factory: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update configuration
    UpdateConfig {
        controller: Option<String>,
        performance_fee: Option<Decimal>,
    },
    /// Deposit UST to anchor protocol
    DepositAnchor {
        amount: Uint128,
    },
    /// Withdraw UST from anchor protocol
    WithdrawAnchor {
        aterra_amount: Option<Uint128>,
    },
    /// Deposit UST to mirror staking
    DepositMirror {
        amount: Uint128,
        mirror_asset_addr: String,
    },
    DepositMirrorHook {
        mirror_asset_addr: String,
    },
    /// Withdraw UST from mirror staking
    WithdrawMirror {
        mirror_lp_amount: Uint128,
        mirror_asset_addr: String,
    },
    WithdrawMirrorHook {
        mirror_asset_addr: String,
    },
    /// Claim MIR reward and swap to UST
    CompoundMirror {},
    CompoundMirrorHook {},
    /// Open short position
    OpenShortPosition {
        aterra_amount: Uint128,
        collateral_ratio: Decimal,
        mirror_asset_addr: String,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
    },
    CloseShortPosition {
        position_idx: Uint128,
    },
    // CheckShortPosition {
    //     position_idx: Uint128,
    //     collateral_ratio: Decimal,
    //     mirror_asset_addr: String,
    // },
    /// Withdraw all invested UST from anchor and mirror protocol
    WithdrawAll {},
    /// Withdraw some UST from invested
    /// TODO check
    // WithdrawInvested {
    //     amount: Uint128,
    // },
    /// Send UST to vault
    WithdrawToVault {
        amount: Option<Uint128>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query current configuration
    Config {},
    /// Query current strategy state
    State {},
    /// Query total underlying balance in strategy
    TotalBalance {},
    // /// Query idle underlying balance
    // IdleBalance {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub contract_addr: String,
    pub controller: String,
    pub vault: String,
    pub performance_fee: Decimal,
    pub stable_denom: String,

    pub anchor_market: String,
    pub aterra_contract: String,

    pub mirror_token: String,
    pub mirror_staking: String,
    pub mirror_mint: String,
    pub mirror_oracle: String,
    pub terraswap_factory: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub anchor_deposited: Uint128,
    pub aterra_collateral: Uint128,
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Controller address
    pub controller: String,
    /// asset denomination
    pub stable_denom: String,
    /// Invest percentage
    pub invest_percentage: Decimal,
    /// minimum lock period
    pub lock_period: u64,

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update configuration
    UpdateConfig {
        controller: Option<String>,
        strategy: Option<String>,
        invest_percentage: Option<Decimal>,
        lock_period: Option<u64>,
        force_withdraw: Option<bool>,
    },
    /// Deposit asset
    Deposit {},
    /// Withdraw asset
    Withdraw {
        withdraw_amount: Uint128,
        force_withdraw: bool,
    },
    /// Claim yield
    ClaimYield {},
    /// Claim principal
    ClaimPrincipal {},
    /// Invest underlying to strategy
    Invest {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query current configuration
    Config {},
    /// Query deposit info
    DepositInfo { addr: String },
    /// Query current underlying balance in vault
    VaultBalance {},
    /// Query current underlying balance in vault and strategy
    TotalBalance {},
    /// Query current underlying balance in vault
    Available {},
    /// Query current state of vault
    State {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub contract_addr: String,
    pub controller: String,
    pub strategy: Option<String>,
    pub stable_denom: String,
    pub invest_percentage: Decimal,
    pub lock_period: u64,
    pub force_withdraw: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositInfoResponse {
    pub principal: Uint128,
    pub current_amount: Uint128,
    pub share: Uint128,
    pub maturity: u64,
    pub yield_amount: Uint128,
    pub yield_claimed: Uint128,
    pub principal_claimed: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_share: Uint128,
    pub total_subsidized: Uint128,
}

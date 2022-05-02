use cosmwasm_std::{
    to_binary, Addr, Decimal, Deps, Env, QueryRequest, StdResult, Uint128, WasmQuery,
};

use crate::state::Config;
use athena::asset::{Asset, AssetInfo};
use athena::querier::query_supply;
use mirror_protocol::mint::{
    PositionResponse as MirrorPositionResponse, PositionsResponse as MirrorPositionsResponse,
    QueryMsg as MirrorMintQueryMsg,
};
use mirror_protocol::oracle::{
    PriceResponse as MirrorOraclePriceResponse, QueryMsg as MirrorOracleQueryMsg,
};
use mirror_protocol::staking::{
    QueryMsg as MirrorStakingQueryMsg, RewardInfoResponse as MirrorStakingRewardInfoResponse,
    RewardInfoResponseItem as MirrorStakingRewardInfoResponseItem,
};
use terraswap::asset::PairInfo;
use terraswap::factory::QueryMsg as TerraSwapFactoryQueryMsg;
use terraswap::pair::{
    QueryMsg as TerraSwapQueryMsg, ReverseSimulationResponse, SimulationResponse,
};

fn query_mirror_asset_price(
    deps: Deps,
    mirror_oracle: String,
    mirror_asset: String,
    stable_denom: String,
) -> StdResult<Decimal> {
    let mirror_oracle_price_response: MirrorOraclePriceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: mirror_oracle,
            msg: to_binary(&MirrorOracleQueryMsg::Price {
                base_asset: mirror_asset.to_string(),
                quote_asset: stable_denom,
            })?,
        }))?;

    Ok(mirror_oracle_price_response.rate)
}

pub fn query_mirror_staking_rewards(
    deps: Deps,
    config: Config,
) -> StdResult<Vec<MirrorStakingRewardInfoResponseItem>> {
    let mirror_reward_response: MirrorStakingRewardInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&config.mirror_staking)?.to_string(),
            msg: to_binary(&MirrorStakingQueryMsg::RewardInfo {
                staker_addr: deps.api.addr_humanize(&config.contract_addr)?.to_string(),
                asset_token: None,
            })?,
        }))?;
    Ok(mirror_reward_response.reward_infos)
}

pub fn query_mirror_positions(
    deps: Deps,
    env: Env,
    config: Config,
) -> StdResult<Vec<MirrorPositionResponse>> {
    let mirror_position_res: MirrorPositionsResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&config.mirror_mint)?.to_string(),
            msg: to_binary(&MirrorMintQueryMsg::Positions {
                owner_addr: Some(env.contract.address.to_string()),
                asset_token: None,
                start_after: None,
                limit: None,
                order_by: None,
            })?,
        }))?;
    Ok(mirror_position_res.positions)
}

pub fn get_mirror_balance(deps: Deps, config: Config) -> StdResult<Uint128> {
    let reward_infos = query_mirror_staking_rewards(deps, config.clone())?;
    let mut total_staked_in_denom = Uint128::zero();

    for reward_info in reward_infos {
        if reward_info.bond_amount.is_zero() || reward_info.is_short {
            continue;
        }

        let mirror_asset = AssetInfo::Token {
            contract_addr: reward_info.asset_token.clone(),
        };
        let pair_info = query_pair_info(
            deps,
            deps.api
                .addr_humanize(&config.terraswap_factory)?
                .to_string(),
            config.get_underlying(),
            mirror_asset.clone(),
        )?;

        let denom_balance = config.get_underlying().query_balance(
            &deps.querier,
            deps.api,
            Addr::unchecked(pair_info.contract_addr.to_string()),
        )?;

        let m_asset_balance = mirror_asset.query_balance(
            &deps.querier,
            deps.api,
            Addr::unchecked(pair_info.contract_addr.to_string()),
        )?;
        let total_denom_balance = m_asset_balance
            * query_mirror_asset_price(
                deps,
                deps.api.addr_humanize(&config.mirror_oracle)?.to_string(),
                reward_info.asset_token,
                config.stable_denom.clone(),
            )?
            + denom_balance;
        let lp_total_supply = query_supply(
            &deps.querier,
            Addr::unchecked(pair_info.liquidity_token.to_string()),
        )?;
        let staked_amount_in_denom =
            total_denom_balance * Decimal::from_ratio(reward_info.bond_amount, lp_total_supply);
        total_staked_in_denom = total_staked_in_denom + staked_amount_in_denom;
    }
    Ok(total_staked_in_denom)
}

pub fn query_pair_info(
    deps: Deps,
    terraswap_factory: String,
    asset0: AssetInfo,
    asset1: AssetInfo,
) -> StdResult<PairInfo> {
    let pair_info: PairInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: terraswap_factory,
        msg: to_binary(&TerraSwapFactoryQueryMsg::Pair {
            asset_infos: [asset0.into(), asset1.into()],
        })?,
    }))?;

    Ok(pair_info)
}

pub fn simulate_swap(deps: Deps, offer_asset: Asset, swap_pair: String) -> StdResult<Uint128> {
    let simulate_response: SimulationResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: swap_pair,
            msg: to_binary(&TerraSwapQueryMsg::Simulation {
                offer_asset: offer_asset.into(),
            })?,
        }))?;

    Ok(simulate_response.return_amount)
}

pub fn reserve_simulate_swap(
    deps: Deps,
    ask_asset: Asset,
    swap_pair: String,
) -> StdResult<ReverseSimulationResponse> {
    let simulate_response: ReverseSimulationResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: swap_pair,
            msg: to_binary(&TerraSwapQueryMsg::ReverseSimulation {
                ask_asset: ask_asset.into(),
            })?,
        }))?;

    Ok(simulate_response)
}

pub fn get_lp_value(
    deps: Deps,
    terraswap_factory: String,
    underlying: AssetInfo,
    mirror_asset: AssetInfo,
    amount: Uint128,
) -> StdResult<Uint128> {
    let pair_info = query_pair_info(
        deps,
        terraswap_factory,
        underlying.clone(),
        mirror_asset.clone(),
    )?;

    let pool_stable_amount = underlying.query_balance(
        &deps.querier,
        deps.api,
        Addr::unchecked(pair_info.contract_addr.to_string()),
    )?;

    let pool_mirror_amount = mirror_asset.query_balance(
        &deps.querier,
        deps.api,
        Addr::unchecked(pair_info.contract_addr.to_string()),
    )?;

    let lp_total_supply = query_supply(
        &deps.querier,
        Addr::unchecked(pair_info.liquidity_token.to_string()),
    )?;

    let stable_value = pool_stable_amount * Decimal::from_ratio(amount, lp_total_supply);
    let mirror_amount = pool_mirror_amount * Decimal::from_ratio(amount, lp_total_supply);

    let mirror_value = simulate_swap(
        deps,
        Asset {
            info: mirror_asset,
            amount: mirror_amount,
        },
        pair_info.contract_addr.clone(),
    )?;

    let underlying_asset = Asset {
        info: underlying,
        amount: stable_value,
    };

    Ok(mirror_value + underlying_asset.deduct_tax(&deps.querier)?.amount)
}

pub fn query_mirror_position(
    deps: Deps,
    config: Config,
    position_idx: Uint128,
) -> StdResult<MirrorPositionResponse> {
    let mirror_position_response: MirrorPositionResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&config.mirror_mint)?.into(),
            msg: to_binary(&MirrorMintQueryMsg::Position { position_idx })?,
        }))?;
    Ok(mirror_position_response)
}

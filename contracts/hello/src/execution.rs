use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult, Uint128};
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQuerier, InjectiveQueryWrapper, MarketId};
use injective_math::FPDecimal;

use crate::ContractError;
use crate::state::{BMarket, BOrder, BRewardPool, Direction, MARKETS, my_pool_assets, orders, MyRewardPoolAsset, POOLS, STATE, Status};

pub fn open_market(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    id: String,
    commission_rate: u32,
    partner_rate: u32,
    long_odd: u32,
    short_odd: u32,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let conf = STATE.load(deps.storage).expect("Contract not found");
    if conf.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let querier = InjectiveQuerier::new(&deps.querier);
    let market_id = MarketId::new(id.clone()).expect("create market Id");
    if let Ok(market) = querier.query_derivative_market(&market_id) {
        // update function for new or existing keys
        let update = |d: Option<BMarket>| -> StdResult<BMarket> {
            let pair = market.market.market.unwrap();
            match d {
                Some(_one) => Err(StdError::generic_err("Duplicated Market")),
                None => Ok(BMarket {
                    id: id.clone(),
                    base: pair.oracle_base,
                    quote: pair.oracle_quote,
                    commission_rate,
                    partner_rate,
                    long_odd,
                    short_odd,
                    current_price: market.market.mark_price,
                    last_price: market.market.mark_price,
                }),
            }
        };

        let r = MARKETS.update(deps.storage, &id, update);

        match r {
            Ok(_) => Ok(Response::new()
                .add_attribute("method", "execute")
                .add_attribute("owner", info.sender)
                .add_attribute("market_id", &id)),
            Err(e) => Err(ContractError::Std(e))
        }
    } else {
        Err(ContractError::Std(StdError::generic_err("Not found derivative market")))
    }
}

pub fn create_pool(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    symbol: String,
    denom: String,
    _max_depositor: u32,
    _num_of_depositor: u32,
    _min_deposit_amount: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let conf = STATE.load(deps.storage).expect("Contract not found");
    if conf.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let id = format!("P{}", env.block.height);

    let new_pool = BRewardPool {
        id: id.clone(),
        symbol,
        denom,
        balance: Uint128::from(0u32),
        dev_rewards: Uint128::from(0u32),
        pool_token_supply: Uint128::from(0u32),
    };

    POOLS.save(deps.storage, &id.as_str(), &new_pool).expect("Save error!");

    Ok(Response::new()
        .add_attribute("method", "execute")
        .add_attribute("owner", info.sender)
        .add_attribute("pool_id", &id))
}

//
// deposit:
// LP_tokens_received = (tokens_deposited * pool_token_supply) / balance
pub fn deposit(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    pool_id: String,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {

    // check if pool exists
    if let Some(pool) = POOLS.may_load(deps.storage, &pool_id)? {
        // check if pool tokens deposited
        if let Some(deposited) = info.funds.iter().find(|e| e.denom == pool.denom) {
            // caclulate amount of LP tokens.
            let balance = if pool.balance.u128() > 0 { pool.balance } else { Uint128::from(1u32) };
            let supply = if pool.pool_token_supply.u128() > 0 { pool.pool_token_supply } else { Uint128::from(1u32) };
            let lp_received = deposited.amount * supply / balance;

            // add LP tokens amount
            // list all user's pool assets
            let my_assets: StdResult<Vec<MyRewardPoolAsset>> = my_pool_assets().idx.owner
                .prefix(info.sender.clone())
                .range(deps.storage, None, None, Order::Ascending)
                .map(|r| {
                    r.map(|(_, e)| e)
                })
                .collect();
            // find the asset by the given pool id
            let assetid = match my_assets.unwrap().iter().find(|pa| pa.pool_id == pool.id) {
                Some(pa) => pa.id.clone(),
                None => {
                    let key = sha256::digest(format!("{}{}", env.block.height, info.sender.as_str()));
                    key.split_at(20).0.to_string()
                }
            };

            let update_pool = |d: Option<BRewardPool>| -> StdResult<BRewardPool> {
                match d {
                    Some(mut one) => {
                        one.balance += deposited.amount;
                        one.pool_token_supply += lp_received;
                        Ok(one)
                    }
                    None => Err(StdError::not_found("Deposit not found")),
                }
            };
            POOLS.update(deps.storage, &pool_id, update_pool).expect("save error");

            let update_my_asset = |d: Option<MyRewardPoolAsset>| -> StdResult<MyRewardPoolAsset> {
                match d {
                    Some(mut one) => {
                        one.amount += deposited.amount;
                        one.pool_token_amount += lp_received;
                        Ok(one)
                    }
                    None => {
                        let key = sha256::digest(format!("{}{}", env.block.height, info.sender.as_str()));
                        let id = key.split_at(20).0;
                        let new_deposit = MyRewardPoolAsset {
                            id: id.to_string(),
                            depositor: info.sender.clone(),
                            pool_id: pool_id.clone(),
                            pool_token_amount: lp_received,
                            amount: deposited.amount,
                        };
                        Ok(new_deposit)
                    }
                }
            };
            my_pool_assets().update(deps.storage, &assetid, update_my_asset).expect("save error");
        }

        return Ok(Response::new()
            .add_attribute("method", "execute")
            .add_attribute("action", "deposit")
            .add_attribute("owner", &info.sender)
            // .add_attribute("amount", info.funds)
            .add_attribute("pool_id", &pool_id));
    }

    Err(ContractError::Std(StdError::generic_err("Failed")))
}

// withdraw: tokens_received = (LP_tokens_withdrawn * pool_tokens) / total_pool_tokens
pub fn withdraw(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    pool_id: String,
    amount: Uint128, // LP amount to withdraw
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {

    // check if pool exists
    if let Some(pool) = POOLS.may_load(deps.storage, &pool_id)? {
        if pool.balance < amount {
            return Err(ContractError::Std(StdError::generic_err("insufflation fund")));
        }
        let my_deposits: StdResult<Vec<MyRewardPoolAsset>> = my_pool_assets().idx.owner
            .prefix(info.sender.clone())
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| r.map(|(_, e)| e))
            .collect();

        if let Some(asset) = my_deposits.unwrap().iter().find(|e| e.pool_id == pool_id) {
            let tokens_received = (amount * pool.balance) / pool.pool_token_supply;
            if asset.amount == amount {
                my_pool_assets().remove(deps.storage, &asset.id).expect("remove error");
            } else {
                let update = |d: Option<MyRewardPoolAsset>| -> StdResult<MyRewardPoolAsset> {
                    match d {
                        Some(mut one) => {
                            one.amount -= tokens_received;
                            one.pool_token_amount -= amount;
                            Ok(one)
                        }
                        None => Err(StdError::not_found("Deposit not found")),
                    }
                };
                my_pool_assets().update(deps.storage, &asset.id, update).expect("update error");
            };

            let update = |d: Option<BRewardPool>| -> StdResult<BRewardPool> {
                match d {
                    Some(mut one) => {
                        one.pool_token_supply -= amount;
                        one.balance -= tokens_received;
                        Ok(one)
                    }
                    None => Err(StdError::not_found("Deposit not found")),
                }
            };
            POOLS.update(deps.storage, &pool_id, update).expect("save error");

            // withdraw rewards
            let withdraw_msg = BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin::new( tokens_received.u128(), pool.denom)],
            };

            return Ok(Response::new()
                .add_message(withdraw_msg)
                .add_attribute("method", "execute")
                .add_attribute("action", "deposit")
                .add_attribute("owner", &info.sender)
                // .add_attribute("amount", &info.funds)
                .add_attribute("pool_id", &pool_id));
        }
    }

    Err(ContractError::Std(StdError::generic_err("failed")))
}

pub fn place_order(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    market_id: String,
    pool_id: String,
    direction: Direction,
    open_time: u64,
    close_time: u64,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    // check if pool exists
    let market = MARKETS.load(deps.storage, &market_id)?;

    if let Some(pool) = POOLS.may_load(deps.storage, &pool_id)? {
        let querier = InjectiveQuerier::new(&deps.querier);
        let market_id_object = MarketId::new(market_id.clone()).expect("create market Id");
        let open_price = querier.query_derivative_market(&market_id_object)?.market.mark_price;
        // check if pool tokens deposited
        if let Some(amount) = info.funds.iter().find(|e| e.denom == pool.denom) {
            let odd = if direction.clone() == Direction::LONG { market.long_odd } else { market.short_odd };
            if amount.amount > Uint128::new(0) {
                let order = BOrder {
                    owner: info.sender.clone(),
                    market_id,
                    pool_id,
                    direction,
                    open_time,
                    close_time,
                    open_price,
                    close_price: FPDecimal::default(),
                    odd,
                    amount: amount.clone(),
                    status: Status::PENDING,
                    reward: Default::default(),
                };
                let key = sha256::digest(format!("{}{}{}", env.block.height, info.sender.as_str(), open_time));
                let id = key.split_at(20).0;
                orders().save(deps.storage, &id, &order).expect("created order");

                // add order to live queue
                STATE.update(deps.storage, |mut a| -> StdResult<_> {
                    a.live_orders.push(id.clone().to_string());
                    Ok(a)
                }).expect("states not found");
                // STATE.save(deps.storage, conf);

                return Ok(Response::new()
                    .add_attribute("method", "instantiate")
                    .add_attribute("owner", info.sender));
            }
        }
    }
    Err(ContractError::Std(StdError::generic_err("error")))
}


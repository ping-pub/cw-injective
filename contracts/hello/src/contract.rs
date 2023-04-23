use std::borrow::BorrowMut;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult, to_binary, Uint128};
use cw2::set_contract_version;

use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQuerier, InjectiveQueryWrapper, MarketId};
use injective_math::FPDecimal;

use crate::error::ContractError;
use crate::execution::{create_pool, deposit, open_market, place_order, withdraw};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg};
use crate::query::{markets, my_orders, my_pools, pools};
use crate::state::{BMarket, BOrder, BRewardPool, ContractConfigState, Direction, MARKETS, orders, STATE, POOLS, Status};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:becole-battle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let state = ContractConfigState {
        owner: info.sender.clone(),
        live_orders: vec![],
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    match msg {
        ExecuteMsg::PlaceOrder { market_id, pool_id, direction, open_time, close_time }
        => {
            let dir = if direction == 1 { Direction::LONG } else { Direction::SHORT };
            place_order(deps, env, info, market_id, pool_id, dir, open_time, close_time)
        }
        ExecuteMsg::Deposit { pool_id } => deposit(deps, env, info, pool_id),
        ExecuteMsg::Withdraw { pool_id, amount } => withdraw(deps, env, info, pool_id, Uint128::from(amount)),
        ExecuteMsg::CreatePool { symbol, denom, /*max_depositor,num_of_depositor, min_deposit_amount */ }
        => create_pool(deps, env, info, symbol, denom, 0, 0, Uint128::from(0u32)),
        ExecuteMsg::OpenMarket { id, commission_rate, partner_rate, long_odd, short_odd }
        => open_market(deps, env, info, id, commission_rate, partner_rate, long_odd, short_odd),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps<InjectiveQueryWrapper>,
    _env: Env,
    msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => {
            let store = STATE.load(deps.storage);
            let owner = store.expect("State should exists").owner;
            to_binary(owner.as_str())
        }
        QueryMsg::Markets {} => markets(deps),
        QueryMsg::Pools {} => pools(deps),
        QueryMsg::MyPools { owner } => my_pools(deps, owner),
        QueryMsg::MyOrders { owner } => my_orders(deps, owner)
    }
}

#[entry_point]
pub fn sudo(deps: DepsMut<InjectiveQueryWrapper>, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BeginBlocker {} => {
            let querier = InjectiveQuerier::new(&deps.querier);
            let mut msgs: Vec<BankMsg> = vec![];

            // processing live orders
            let conf = STATE.load(deps.storage).expect("load config error");
            let mut new_queue: Vec<String> = vec![];
            conf.live_orders.iter().for_each(|id| {
                if let Ok(order) = orders().load(deps.storage, id) {
                    let market_id = MarketId::new(&order.market_id.clone()).expect("create market Id");
                    if let Ok(market) = querier.query_derivative_market(&market_id) {
                        if market.market.mark_price.gt(&FPDecimal::zero()) {
                            let price = &market.market.mark_price;
                            let mut win = false;
                            let mut reward = Coin::default();
                            if order.close_time <= env.block.time.seconds() {
                                let update = |d: Option<BOrder>| -> StdResult<BOrder> {
                                    match d {
                                        Some(mut one) => {
                                            match &one.direction {
                                                Direction::LONG => {
                                                    one.close_price = price.clone();
                                                    if &order.open_price >= price {
                                                        one.status = Status::LOSE;
                                                        one.reward = Coin {
                                                            denom: one.amount.denom.clone(),
                                                            amount: one.amount.amount,
                                                        };
                                                        win = false;
                                                    } else {
                                                        one.status = Status::WIN;
                                                        one.reward = Coin {
                                                            denom: one.amount.denom.clone(),
                                                            amount: one.amount.amount * Uint128::from(order.odd) / Uint128::from(100u32),
                                                        };
                                                        reward = one.reward.clone();
                                                        win = true;
                                                    }
                                                }
                                                Direction::SHORT => {
                                                    one.close_price = price.clone();
                                                    if &order.open_price <= price {
                                                        one.status = Status::LOSE;
                                                        one.reward = Coin {
                                                            denom: one.amount.denom.clone(),
                                                            amount: one.amount.amount,
                                                        };
                                                        win = false;
                                                    } else {
                                                        one.status = Status::WIN;
                                                        one.reward = Coin {
                                                            denom: one.amount.denom.clone(),
                                                            amount: one.amount.amount * Uint128::from(order.odd) / Uint128::from(100u32),
                                                        };
                                                        win = true;
                                                        reward = one.reward.clone();
                                                    }
                                                }
                                            };
                                            Ok(one)
                                        }
                                        None => Err(StdError::generic_err("Not found")),
                                    }
                                };
                                orders().update(deps.storage, &id, update).expect("failed to update order");

                                if win {
                                    // send the principal and reward to owner of the order
                                    msgs.push(BankMsg::Send {
                                        to_address: order.owner.to_string(),
                                        amount: vec![Coin::new(order.amount.amount.u128() + reward.amount.u128(), order.amount.denom)],
                                    });

                                    // sub amount from reward pool
                                    let update_pool = |opt: Option<BRewardPool>| -> StdResult<BRewardPool> {
                                        match opt {
                                            Some(mut p) => {
                                                p.balance -= reward.amount.clone();
                                                Ok(p)
                                            },
                                            None => Err(StdError::generic_err("Pool does not exists"))
                                        }
                                    };
                                    let _ = POOLS.update(deps.storage, &order.pool_id, update_pool);
                                } else {
                                    // add amount to reward pool
                                    let update_pool = |opt: Option<BRewardPool>| -> StdResult<BRewardPool> {
                                        match opt {
                                            Some(mut p) => {
                                                p.balance += order.amount.amount / Uint128::from(2u32);
                                                p.dev_rewards += order.amount.amount / Uint128::from(2u32);
                                                Ok(p)
                                            },
                                            None => Err(StdError::generic_err("Pool does not exists"))
                                        }
                                    };
                                    let _ = POOLS.update(deps.storage.borrow_mut(), &order.pool_id, update_pool);
                                }
                            }
                        } else {
                            // remove all closed order from the living orders.
                            // constructs a new living orders queue
                            new_queue.push(id.clone())
                        }
                        // clean live queue
                        STATE.update(deps.storage, |mut a| -> StdResult<_> {
                            a.live_orders = new_queue.to_owned();
                            Ok(a)
                        }).expect("states not found");
                    }
                }
            });

            // update market lucky value every 1000 blocks
            if env.block.height % 1000 == 0 {
                let markets: StdResult<Vec<BMarket>> = MARKETS
                    .range(deps.storage, None, None, Order::Ascending)
                    .map(|r| {
                        r.map(|(_, e)| e)
                    })
                    .collect();

                markets.unwrap().iter().for_each(|m| {
                    let querier = InjectiveQuerier::new(&deps.querier);
                    let market_id = MarketId::new(m.id.clone()).expect("create market Id");
                    if let Ok(market_response) = querier.query_derivative_market(&market_id) {
                        let market = market_response.market;
                        let update = |d: Option<BMarket>| -> StdResult<BMarket> {
                            match d {
                                Some(mut one) => {
                                    let lucky = (env.block.time.seconds() as u64 % 10) as u32;

                                    println!("lucky: {}", lucky);

                                    one.last_price = one.current_price;
                                    one.current_price = market.mark_price;
                                    if one.current_price.num > one.last_price.num {
                                        one.long_odd = 72 - lucky;
                                        one.short_odd = 70 + lucky;
                                    } else {
                                        one.long_odd = 72 + lucky;
                                        one.short_odd = 70 - lucky;
                                    }
                                    Ok(one)
                                }
                                None => Err(StdError::generic_err("Not found")),
                            }
                        };
                        let _ = MARKETS.update(deps.storage, &m.id, update);
                    }
                });
            }

            Ok(Response::new().add_messages(msgs))
        }
    }
}



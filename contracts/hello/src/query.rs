use cosmwasm_std::{Addr, Binary, Deps, Order, StdError, StdResult, to_binary};
use injective_cosmwasm::InjectiveQueryWrapper;
use crate::msg::{MarketsResponse, MyPoolResponse, OrdersResponse, PoolsResponse};

use crate::state::{MARKETS, POOLS, BMarket, BRewardPool, MyRewardPoolAsset, orders, BOrder, my_pool_assets};

pub fn markets(deps: Deps<InjectiveQueryWrapper>,) -> StdResult<Binary> {
    let markets:  StdResult<Vec<BMarket>> = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            r.map(|(_, e)| e)
        })
        .collect();
    match markets {
        Ok(m) => to_binary(&MarketsResponse{
            markets: m
        }),
        Err(_) => Err(StdError::not_found("markets not found")),
    }
}

pub fn pools(deps: Deps<InjectiveQueryWrapper>,) -> StdResult<Binary> {
    let pools:  StdResult<Vec<BRewardPool>> = POOLS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            r.map(|(_, e)| e)
        })
        .collect();
    match pools {
        Ok(m) => to_binary(&PoolsResponse{pools: m}),
        Err(_) => Err(StdError::not_found("markets not found")),
    }
}

pub fn my_pools(deps: Deps<InjectiveQueryWrapper>, owner: Addr) -> StdResult<Binary> {
    let pool:  StdResult<Vec<MyRewardPoolAsset>> = my_pool_assets().idx.owner
        .prefix(owner)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            r.map(|(_, e)| e)
        })
        .collect();
    match pool {
        Ok(m) => to_binary( &MyPoolResponse{deposits: m}),
        Err(_) => Err(StdError::not_found("markets not found")),
    }
}

pub fn my_orders(deps: Deps<InjectiveQueryWrapper>, owner: Addr) -> StdResult<Binary> {
    let result: Result<Vec<BOrder>, _> = orders().idx.owner
        .prefix(owner)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            r.map(|(_, e)| e)
        })
        .collect();
    match result {
        Ok(m) => to_binary(&OrdersResponse{orders: m}),
        Err(_) => Err(StdError::not_found("markets not found")),
    }
}

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use crate::state::{BMarket, BRewardPool, BOrder, MyRewardPoolAsset};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    // 选择一个INJ市场的ID，设置赔率，奖金池【{denom: USDT, amount: 1000000}】
    OpenMarket {
        id: String,
        commission_rate: u32,
        partner_rate: u32,
        long_odd: u32,
        short_odd: u32,
    },
    CreatePool {
        symbol: String,
        denom: String,
    },
    Deposit {
        pool_id: String
    },
    Withdraw {
        pool_id: String,
        amount: Uint128,
    },
    PlaceOrder {
        market_id: String,
        pool_id: String,
        direction: i32,
        open_time: u64,
        close_time: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    Owner {},
    #[returns(MarketsResponse)]
    Markets {},
    #[returns(PoolsResponse)]
    Pools {},
    #[returns(MyPoolResponse)]
    MyPools {
        owner: Addr
    },
    #[returns(OrdersResponse)]
    MyOrders {
        owner: Addr,
    },
}

#[cw_serde]
pub enum SudoMsg {
    BeginBlocker {},
}

#[cw_serde]
pub struct MarketsResponse {
    pub markets: Vec<BMarket>,
}

#[cw_serde]
pub struct PoolsResponse {
    pub pools: Vec<BRewardPool>,
}

#[cw_serde]
pub struct MyPoolResponse {
    pub deposits: Vec<MyRewardPoolAsset>,
}

#[cw_serde]
pub struct OrdersResponse {
    pub orders: Vec<BOrder>,
}


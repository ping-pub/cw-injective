use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Index, IndexedMap, IndexList, Item, Map, MultiIndex};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use injective_math::FPDecimal;

pub const STATE: Item<ContractConfigState> = Item::new("state");
pub const MARKETS: Map<&str, BMarket> = Map::new("markets");
pub const POOLS: Map<&str, BRewardPool> = Map::new("pools");

pub struct MyPoolIndexes<'a> {
    pub owner: MultiIndex<'a, Addr, MyRewardPoolAsset, String>,
    pub pool: MultiIndex<'a, String, MyRewardPoolAsset, String>,
}

impl<'a> IndexList<MyRewardPoolAsset> for MyPoolIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<MyRewardPoolAsset>> + '_> {
        let v: Vec<&dyn Index<MyRewardPoolAsset>> = vec![&self.pool, &self.owner];
        Box::new(v.into_iter())
    }
}

pub fn my_pool_assets<'a>() -> IndexedMap<'a, &'a str, MyRewardPoolAsset, MyPoolIndexes<'a>> {
    let indexes = MyPoolIndexes {
        owner: MultiIndex::new(
            |_, d: &MyRewardPoolAsset| d.depositor.clone(),
            "myPool",
            "myPool__owner",
        ),
        pool: MultiIndex::new(
            |_, d: &MyRewardPoolAsset| d.pool_id.clone(),
            "myPool",
            "myPool__pool_id",
        ),
    };
    IndexedMap::new("myPool", indexes)
}

pub struct MyOrderIndexes<'a> {
    pub owner: MultiIndex<'a, Addr, BOrder, String>,
}

impl<'a> IndexList<BOrder> for MyOrderIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<BOrder>> + '_> {
        let v: Vec<&dyn Index<BOrder>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn orders<'a>() -> IndexedMap<'a, &'a str, BOrder, MyOrderIndexes<'a>> {
    let indexes = MyOrderIndexes {
        owner: MultiIndex::new(
            |_, d: &BOrder| d.owner.clone(),
            "orders",
            "orders__owner",
        ),
    };
    IndexedMap::new("orders", indexes)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractConfigState {
    pub owner: Addr,
    pub live_orders: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BMarket {
    pub id: String,
    pub base: String,
    pub quote: String,
    pub commission_rate: u32,
    pub partner_rate: u32,
    pub long_odd: u32,
    pub short_odd: u32,
    pub current_price: FPDecimal,
    pub last_price: FPDecimal,
}

/*
deposit:  LP_tokens_received = (tokens_deposited * balance) / pool_token_supply
withdraw: tokens_received = (LP_tokens_withdrawn * pool_tokens_supply) / balance
*/
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BRewardPool {
    pub id: String,
    pub symbol: String,
    pub denom: String,
    pub balance: Uint128,
    pub pool_token_supply: Uint128,
    pub dev_rewards: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MyRewardPoolAsset {
    pub id: String,
    pub depositor: Addr,
    pub pool_id: String,
    pub amount: Uint128,
    pub pool_token_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum Direction {
    LONG,
    SHORT,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum Status {
    PENDING,
    WIN,
    LOSE,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BOrder {
    pub owner: Addr,
    pub market_id: String,
    pub pool_id: String,
    pub direction: Direction,
    pub open_price: FPDecimal,
    pub close_price: FPDecimal,
    pub odd: u32,
    pub amount: Coin,
    pub status: Status,
    pub reward: Coin, // including amount
    pub open_time: u64,
    pub close_time: u64,
}



use std::marker::PhantomData;
use std::str::FromStr;

use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{coins, to_binary, Addr, Api, BlockInfo, ContractInfo, ContractResult, CustomQuery, DepsMut, Env, OwnedDeps, Querier, QuerierResult, QuerierWrapper, Storage, SystemResult, Timestamp, TransactionInfo, Uint128, from_binary};

use injective_cosmwasm::{
    HandlesMarketIdQuery, InjectiveQueryWrapper, MarketId, SpotMarket, SpotMarketResponse, WasmMockQuerier,
};
use injective_math::FPDecimal;

use crate::contract::{execute, instantiate, query, sudo};
use crate::msg::{ExecuteMsg, InstantiateMsg, MarketsResponse, MyPoolResponse, OrdersResponse, PoolsResponse, QueryMsg, SudoMsg};

pub const TEST_CONTRACT_ADDR: &str = "inj14hj2tavq8fpesdwxxcu44rty3hh90vhujaxlnz";

pub const POOL_ID: &str = "P12345000";

pub fn inj_mock_env() -> Env {
    Env {
        block: BlockInfo {
            height: 12_345_000,
            time: Timestamp::from_nanos(1_571_797_425_879_305_532),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        transaction: Some(TransactionInfo { index: 3 }),
        contract: ContractInfo {
            address: Addr::unchecked(TEST_CONTRACT_ADDR),
        },
    }
}

pub trait OwnedDepsExt<S, A, Q, C>
where
    C: CustomQuery,
{
    fn as_mut_deps(&mut self) -> DepsMut<C>;
}

impl<S, A, Q, C> OwnedDepsExt<S, A, Q, C> for OwnedDeps<S, A, Q, C>
where
    S: Storage,
    A: Api,
    Q: Querier,
    C: CustomQuery,
{
    fn as_mut_deps(&mut self) -> DepsMut<C> {
        return DepsMut {
            storage: &mut self.storage,
            api: &self.api,
            querier: QuerierWrapper::new(&self.querier),
        };
    }
}

pub fn inj_mock_deps() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier, InjectiveQueryWrapper> {
    let mut custom_querier: WasmMockQuerier = WasmMockQuerier::new();
    custom_querier.spot_market_response_handler = Some(Box::new(create_spot_market_handler()));
    OwnedDeps {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
        custom_query_type: PhantomData::default(),
    }
}

#[test]
fn proper_initialization() {
    let sender_addr = "inj1x2ck0ql2ngyxqtw8jteyc0tchwnwxv7npaungt";
    let msg = InstantiateMsg {};
    let info = mock_info(sender_addr, &coins(1000, "earth"));

    let env = inj_mock_env();
    let mut deps = inj_mock_deps();

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut_deps(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let q = query(deps.as_ref(), env.clone(), QueryMsg::Owner{});
    // assert_eq!(q.unwrap().clone(), "sender_addr");
    println!("addr: {}", q.unwrap().to_string())
}


#[test]
fn test_queries() {
    let sender_addr = "inj1x2ck0ql2ngyxqtw8jteyc0tchwnwxv7npaungt";
    let msg = InstantiateMsg {};
    let info = mock_info(sender_addr, &coins(1000, "earth"));

    let env = inj_mock_env();
    let mut deps = inj_mock_deps();
    let market_id = "0x54d4505adef6a5cef26bc403a33d595620ded4e15b9e2bc3dd489b714813366a";

    // initial
    let res = instantiate(deps.as_mut_deps(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    execute(deps.as_mut_deps(), env.clone(), info.clone(),ExecuteMsg::OpenMarket {
        id: market_id.to_string(),
        commission_rate: 50,
        partner_rate: 10,
        long_odd: 80,
        short_odd: 80,
    }).expect("failed");

    let market = query(deps.as_ref(), env.clone(), QueryMsg::Markets{});
    match market {
        Ok(t) => {
            let output: MarketsResponse = from_binary(&t).unwrap();
            println!("=={:?}===", output.markets)
        },
        Err(_) => println!("error")
    }

    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::CreatePool {
        symbol: "USDT".to_string(),
        denom: "earth".to_string(),
        // max_depositor: 100,
        // num_of_depositor: 0,
        // min_deposit_amount:23432,
    }).expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::Pools{}) {
        let output: PoolsResponse = from_binary(&binary).unwrap();
        println!("pool: {:?}", output.pools)
    }

    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::Deposit { pool_id: POOL_ID.to_string()}).expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyPools { owner: info.sender.clone() }) {
        let output: MyPoolResponse = from_binary(&binary).unwrap();
        println!("deposit: {:?}", output.deposits)
    }

    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::Withdraw { pool_id: POOL_ID.to_string(), amount: Uint128::from(100u64) }).expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyPools { owner: info.sender.clone() }) {
        let output: MyPoolResponse = from_binary(&binary).unwrap();
        println!("withdraw: {:?}", output.deposits)
    }

    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::PlaceOrder {
        market_id: market_id.clone().to_string(),
        pool_id: POOL_ID.to_string(),
        direction: 1,
        open_time: Timestamp::default().seconds(),
        close_time: Timestamp::default().seconds()
    })
        .expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyOrders {owner: info.sender.clone()}) {
        let output: OrdersResponse = from_binary(&binary).unwrap();
        println!("Orders: {:?}", output.orders)
    }

    sudo( deps.as_mut_deps(), env.clone(), SudoMsg::BeginBlocker {}).expect("failed on sudo");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyOrders {owner: info.sender.clone()}) {
        let output: OrdersResponse = from_binary(&binary).unwrap();
        println!("close order: {:?}", output.orders)
    }

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::Pools{}) {
        let output: PoolsResponse = from_binary(&binary).unwrap();
        println!("pool: {:?}", output.pools)
    }

    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::Withdraw { pool_id: POOL_ID.to_string(), amount: Uint128::from(100u64) }).expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyPools { owner: info.sender.clone() }) {
        let output: MyPoolResponse = from_binary(&binary).unwrap();
        println!("withdraw: {:?}", output.deposits)
    }

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::Pools{}) {
        let output: PoolsResponse = from_binary(&binary).unwrap();
        println!("pool: {:?}", output.pools)
    }


    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::Deposit { pool_id: POOL_ID.to_string() }).expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyPools { owner: info.sender.clone() }) {
        let output: MyPoolResponse = from_binary(&binary).unwrap();
        println!("deposit: {:?}", output.deposits)
    }

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::Pools{}) {
        let output: PoolsResponse = from_binary(&binary).unwrap();
        println!("pool: {:?}", output.pools)
    }

    execute(deps.as_mut_deps(), env.clone(), info.clone(), ExecuteMsg::Withdraw { pool_id: POOL_ID.to_string(), amount: Uint128::from(642u64) }).expect("failed");

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::MyPools { owner: info.sender.clone() }) {
        let output: MyPoolResponse = from_binary(&binary).unwrap();
        println!("withdraw: {:?}", output.deposits)
    }

    if let Ok(binary) = query(deps.as_ref(), env.clone(), QueryMsg::Pools{}) {
        let output: PoolsResponse = from_binary(&binary).unwrap();
        println!("pool: {:?}", output.pools)
    }

    let market = query(deps.as_ref(), env.clone(), QueryMsg::Markets{});
    match market {
        Ok(t) => {
            let output: MarketsResponse = from_binary(&t).unwrap();
            println!("=={:?}===", output.markets)
        },
        Err(_) => println!("error")
    }

}

// #[test]
// fn test_swap() {
//     let contract_addr = "inj14hj2tavq8fpesdwxxcu44rty3hh90vhujaxlnz";
//     let sender_addr = "inj1x2ck0ql2ngyxqtw8jteyc0tchwnwxv7npaungt";
//     let market_id = MarketId::new(
//         "0x78c2d3af98c517b164070a739681d4bd4d293101e7ffc3a30968945329b47ec6".to_string(),
//     )
//     .expect("failed to create market_id");
//
//     let msg = InstantiateMsg {};
//     let info = mock_info(contract_addr, &coins(1000, "earth"));
//     let mut deps = inj_mock_deps();
//     let env = inj_mock_env();
//     let _ = instantiate(deps.as_mut_deps(), env.clone(), info, msg);
//
//     let info = mock_info(sender_addr, &coins(9000, "usdt"));
//     let msg = ExecuteMsg::SwapSpot {
//         quantity: i32_to_dec(8),
//         price: i32_to_dec(1000),
//     };
//     let res = execute(deps.as_mut_deps(), env.clone(), info, msg).unwrap();
//
//     let expected_atomic_order_message = CreateSpotMarketOrder {
//         sender: env.contract.address.to_owned(),
//         order: SpotOrder {
//             market_id,
//             order_info: OrderInfo {
//                 subaccount_id: SubaccountId::new(
//                     "0xade4a5f5803a439835c636395a8d648dee57b2fc000000000000000000000000"
//                         .to_string(),
//                 )
//                 .expect("failed to create subaccount_id"),
//                 fee_recipient: Some(env.contract.address),
//                 price: i32_to_dec(1000),
//                 quantity: i32_to_dec(8),
//             },
//             order_type: OrderType::BuyAtomic,
//             trigger_price: None,
//         },
//     };
//
//     if let InjectiveMsg::Deposit {
//         sender,
//         subaccount_id: _subaccount_id,
//         amount: _amount,
//     } = &get_message_data(&res.messages, 0).msg_data
//     {
//         assert_eq!(sender.to_string(), contract_addr, "sender not correct")
//     }
//     let order_message = get_message_data(&res.messages, 1);
//     assert_eq!(
//         InjectiveRoute::Exchange,
//         order_message.route,
//         "route was incorrect"
//     );
//     assert_eq!(
//         expected_atomic_order_message, order_message.msg_data,
//         "spot create order had incorrect content"
//     );
//
//     let binary_response = Binary::from_base64("CkIweGRkNzI5MmY2ODcwMzIwOTc2YTUxYTUwODBiMGQ2NDU5M2NhZjE3OWViM2YxOTNjZWVlZGFiNGVhNWUxNDljZWISQwoTODAwMDAwMDAwMDAwMDAwMDAwMBIWMTAwMDAwMDAwMDAwMDAwMDAwMDAwMBoUMzYwMDAwMDAwMDAwMDAwMDAwMDA=").unwrap();
//     let reply_msg = Reply {
//         id: ATOMIC_ORDER_REPLY_ID,
//         result: SubMsgResult::Ok(SubMsgResponse {
//             events: vec![],
//             data: Some(binary_response),
//         }),
//     };
//
//     let transfers_response = reply(deps.as_mut_deps(), inj_mock_env(), reply_msg);
//     let messages = transfers_response.unwrap().messages;
//     assert_eq!(messages.len(), 3);
//
//     if let InjectiveMsg::Withdraw {
//         sender,
//         subaccount_id: _subaccount_id,
//         amount,
//     } = &get_message_data(&messages, 0).msg_data
//     {
//         assert_eq!(sender.to_string(), contract_addr, "sender not correct");
//         assert_eq!(amount.amount, Uint128::from(8u128));
//     } else {
//         panic!("Wrong message type!");
//     }
//
//     if let InjectiveMsg::Withdraw {
//         sender,
//         subaccount_id: _subaccount_id,
//         amount,
//     } = &get_message_data(&messages, 1).msg_data
//     {
//         assert_eq!(sender.to_string(), contract_addr, "sender not correct");
//         assert_eq!(amount.amount, Uint128::from(9000u128 - 8036u128));
//     } else {
//         panic!("Wrong message type!");
//     }
//
//     if let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) = &messages[2].msg {
//         assert_eq!(to_address, sender_addr);
//         assert_eq!(2, amount.len());
//         assert_eq!(amount[0].denom, "INJ");
//         assert_eq!(amount[0].amount, Uint128::from(8u128));
//         assert_eq!(amount[1].denom, "USDT");
//         assert_eq!(amount[1].amount, Uint128::from(9000u128 - 8036u128));
//     } else {
//         panic!("Wrong message type!");
//     }
// }
//
fn create_spot_market_handler() -> impl HandlesMarketIdQuery {
    struct Temp();
    impl HandlesMarketIdQuery for Temp {
        fn handle(&self, market_id: MarketId) -> QuerierResult {
            let response = SpotMarketResponse {
                market: Some(SpotMarket {
                    ticker: "INJ/USDT".to_string(),
                    base_denom: "INJ".to_string(),
                    quote_denom: "USDT".to_string(),
                    maker_fee_rate: FPDecimal::from_str("0.01").unwrap(),
                    taker_fee_rate: FPDecimal::from_str("0.1").unwrap(),
                    relayer_fee_share_rate: FPDecimal::from_str("0.4").unwrap(),
                    market_id,
                    status: 0,
                    min_price_tick_size: FPDecimal::from_str("0.000000000000001").unwrap(),
                    min_quantity_tick_size: FPDecimal::from_str("1000000000000000").unwrap(),
                }),
            };
            SystemResult::Ok(ContractResult::from(to_binary(&response)))
        }
    }
    Temp()
}

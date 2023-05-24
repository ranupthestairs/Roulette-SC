use std::vec;

// use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{
    coins, to_binary, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, Decimal, Empty, Response,
    StdResult, Timestamp, Uint128, WasmMsg,
};

use cw20::{Balance, BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

use crate::{
    msg::{
        BetConfig, BetsInfoResponse, ConfigResponse, Direction, ExecuteMsg, InstantiateMsg,
        QueryMsg, RoomInfoResponse, RoomsInfoResponse,
    },
    state::{AssetInfo, Config, RoomConfig},
};

use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

fn mock_app() -> App {
    App::default()
}

pub fn contract_roulette() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::execute::instantiate,
        crate::query::query,
    );
    Box::new(contract)
}

pub fn cw721_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn init_roulette_contract(router: &mut App) -> Addr {
    // println!("prediction_market_code_id, {:?}", prediction_market_code_id);

    let msg = InstantiateMsg {
        conifg: Config {
            admin: Addr::unchecked("admin"),
            nft_contract: Addr::unchecked("nft_contract"),
            minimum_bet: Uint128::new(1),
            maximum_bet: Uint128::new(1000),
            next_round_seconds: 120,
            distributor: Addr::unchecked("distributor"),
        },
    };
    let roulette_id = router.store_code(contract_roulette());

    let roulette_contract = router
        .instantiate_contract(
            roulette_id,
            Addr::unchecked("admin"),
            &msg,
            &[],
            "roulette",
            Some("admin".to_string()),
        )
        .unwrap();

    roulette_contract
}

fn init_cw20_contract(router: &mut App, roulette_address: &Addr) -> Addr {
    // println!("prediction_market_code_id, {:?}", prediction_market_code_id);

    let msg = Cw20InstantiateMsg {
        name: "Test".to_string(),
        symbol: "Test".to_string(),
        decimals: 6,
        initial_balances: vec![
            Cw20Coin {
                address: "user1".to_string(),
                amount: Uint128::new(10000),
            },
            Cw20Coin {
                address: "user2".to_string(),
                amount: Uint128::new(10000),
            },
            Cw20Coin {
                address: roulette_address.to_string(),
                amount: Uint128::new(10000),
            },
        ],
        mint: None,
        marketing: None,
    };
    let roulette_id = router.store_code(cw20_contract());

    let roulette_contract = router
        .instantiate_contract(
            roulette_id,
            Addr::unchecked("admin"),
            &msg,
            &[],
            "roulette",
            Some("admin".to_string()),
        )
        .unwrap();

    roulette_contract
}

fn mint_gaming_tokens_for_users(router: &mut App, roulette_address: &Addr) -> StdResult<()> {
    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "admin".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(12000),
            }],
        }))
        .unwrap();

    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "user1".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(2000),
            }],
        }))
        .unwrap();

    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "user2".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(2000),
            }],
        }))
        .unwrap();

    router
        .send_tokens(
            Addr::unchecked("admin"),
            roulette_address.clone(),
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(10000),
            }],
        )
        .unwrap();

    Ok(())
}

fn init_two_rooms(
    router: &mut App,
    roulette_address: &Addr,
    token_address: &Addr,
) -> StdResult<()> {
    let msg = ExecuteMsg::AddRoom {
        room_info: RoomConfig {
            room_name: "SEI".to_string(),
            game_denom: AssetInfo::NativeToken {
                denom: "usei".to_string(),
            },
            nft_id: "SEI".to_string(),
        },
    };

    router
        .execute_contract(
            Addr::unchecked("admin"),
            roulette_address.clone(),
            &msg,
            &[],
        )
        .unwrap();

    let msg = ExecuteMsg::AddRoom {
        room_info: RoomConfig {
            room_name: "TEST".to_string(),
            game_denom: AssetInfo::Token {
                contract_addr: token_address.clone(),
            },
            nft_id: "TEST".to_string(),
        },
    };

    router
        .execute_contract(
            Addr::unchecked("admin"),
            roulette_address.clone(),
            &msg,
            &[],
        )
        .unwrap();

    Ok(())
}

#[test]
fn test_update_config() {
    let mut router = mock_app();
    let roulette_address = init_roulette_contract(&mut router);

    let new_config = Config {
        admin: Addr::unchecked("new_admin"),
        nft_contract: Addr::unchecked("nft_contract"),
        minimum_bet: Uint128::new(1),
        maximum_bet: Uint128::new(1000),
        next_round_seconds: 120,
        distributor: Addr::unchecked("distributor"),
    };

    router
        .execute_contract(
            Addr::unchecked("admin"),
            roulette_address.clone(),
            &ExecuteMsg::UpdateConfig { config: new_config },
            &[],
        )
        .unwrap();

    let config: ConfigResponse = router
        .wrap()
        .query_wasm_smart(roulette_address, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(
        config.config,
        Config {
            admin: Addr::unchecked("new_admin"),
            nft_contract: Addr::unchecked("nft_contract"),
            minimum_bet: Uint128::new(1),
            maximum_bet: Uint128::new(1000),
            next_round_seconds: 120,
            distributor: Addr::unchecked("distributor"),
        }
    );
}

#[test]
fn test_add_room() {
    let mut router = mock_app();
    let roulette_address = init_roulette_contract(&mut router);

    let msg = ExecuteMsg::AddRoom {
        room_info: RoomConfig {
            room_name: "SEI".to_string(),
            game_denom: AssetInfo::NativeToken {
                denom: "usei".to_string(),
            },
            nft_id: "SEI".to_string(),
        },
    };

    router
        .execute_contract(
            Addr::unchecked("admin"),
            roulette_address.clone(),
            &msg,
            &[],
        )
        .unwrap();

    let room_info: RoomInfoResponse = router
        .wrap()
        .query_wasm_smart(roulette_address.clone(), &QueryMsg::GetRoom { room_id: 1 })
        .unwrap();

    println!("room_info {:?}", room_info);

    let rooms_info: RoomsInfoResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address,
            &&QueryMsg::GetRooms {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    println!("rooms_info {:?}", rooms_info)
}

#[test]
fn test_bet_with_native_token() {
    let mut router = mock_app();
    let roulette_address = init_roulette_contract(&mut router);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 1,
        bet_info: vec![
            BetConfig {
                direction: Direction::FirstHalf,
                amount: Uint128::new(200),
            },
            BetConfig {
                direction: Direction::SecondHalf,
                amount: Uint128::new(200),
            },
        ],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(400),
            }],
        )
        .unwrap();

    router
        .execute_contract(
            Addr::unchecked("user2"),
            token_address.clone(),
            &Cw20ExecuteMsg::IncreaseAllowance {
                spender: roulette_address.to_string(),
                amount: Uint128::new(200),
                expires: None,
            },
            &[],
        )
        .unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 2,
        bet_info: vec![BetConfig {
            direction: Direction::FirstOfThird,
            amount: Uint128::new(200),
        }],
    };
    router
        .execute_contract(
            Addr::unchecked("user2"),
            roulette_address.clone(),
            &bet_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(200),
            }],
        )
        .unwrap();

    let token_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            token_address,
            &Cw20QueryMsg::Balance {
                address: roulette_address.to_string(),
            },
        )
        .unwrap();

    println!("token_balance, {:?}", token_balance);

    let room_player: BetsInfoResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address.clone(),
            &QueryMsg::GetPlayerInfosForRoom {
                room_id: 2,
                player: Addr::unchecked("user2"),
                start_after: Some(0),
                limit: None,
            },
        )
        .unwrap();

    println!("room player, {:?}", room_player);
}

#[test]
fn test_close_round() {
    let mut router = mock_app();
    let roulette_address = init_roulette_contract(&mut router);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 1,
        bet_info: vec![
            BetConfig {
                direction: Direction::SecondOfThird,
                amount: Uint128::new(100),
            },
            BetConfig {
                direction: Direction::SecondHalf,
                amount: Uint128::new(100),
            },
        ],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(200),
            }],
        )
        .unwrap();

    router
        .execute_contract(
            Addr::unchecked("user1"),
            token_address.clone(),
            &Cw20ExecuteMsg::IncreaseAllowance {
                spender: roulette_address.to_string(),
                amount: Uint128::new(400),
                expires: None,
            },
            &[],
        )
        .unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 2,
        bet_info: vec![
            BetConfig {
                direction: Direction::SecondOfThird,
                amount: Uint128::new(200),
            },
            BetConfig {
                direction: Direction::SecondHalf,
                amount: Uint128::new(200),
            },
        ],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[],
        )
        .unwrap();

    let close_round_msg = ExecuteMsg::CloseRound {};
    router
        .execute_contract(
            Addr::unchecked("distributor"),
            roulette_address.clone(),
            &close_round_msg,
            &[],
        )
        .unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 1,
        bet_info: vec![
            BetConfig {
                direction: Direction::SecondOfThird,
                amount: Uint128::new(100),
            },
            BetConfig {
                direction: Direction::SecondHalf,
                amount: Uint128::new(100),
            },
        ],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(200),
            }],
        )
        .unwrap();

    let bet_info: BetsInfoResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address,
            &QueryMsg::GetPlayerInfosForRoom {
                room_id: 1,
                player: Addr::unchecked("user1"),
                start_after: Some(0),
                limit: None,
            },
        )
        .unwrap();

    println!("bets_info, {:?}", bet_info)
}

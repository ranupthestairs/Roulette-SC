use std::vec;

// use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{
    testing::mock_env, Addr, BlockInfo, Coin, Decimal, Empty, StdResult, Timestamp, Uint128,
    WasmMsg,
};

use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg};
use cw721_base::{
    msg::{ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg},
    MintMsg,
};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

use crate::{
    msg::{
        BetConfig, BetsInfoResponse, ConfigResponse, Direction, ExecuteMsg, InstantiateMsg,
        QueryMsg, RoomInfoResponse, RoomsInfoResponse, RoundOffset, WinnerListResponse,
        WinnerResponse, WithdrawResponse,
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

fn init_roulette_contract(router: &mut App, nft_address: Addr) -> Addr {
    let msg = InstantiateMsg {
        config: Config {
            admin: Addr::unchecked("admin"),
            nft_contract: nft_address,
            next_round_seconds: 120,
            distributor: Addr::unchecked("distributor"),
            platform_fee: Decimal::from_ratio(40 as u128, 100 as u128),
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
            Cw20Coin {
                address: "test_admin".to_string(),
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

fn init_cw721_contract_and_mint(router: &mut App) -> Addr {
    let nft_id = router.store_code(cw721_contract());
    let msg = Cw721InstantiateMsg {
        name: "NFT".to_string(),
        symbol: "NFT".to_string(),
        minter: "admin".to_string(),
    };

    let nft_contract = router
        .instantiate_contract(
            nft_id,
            Addr::unchecked("admin"),
            &msg,
            &[],
            "NFT",
            Some("admin".to_string()),
        )
        .unwrap();

    pub type Extension = Option<Empty>;

    router
        .execute_contract(
            Addr::unchecked("admin"),
            nft_contract.clone(),
            &Cw721ExecuteMsg::<Extension, Extension>::Mint(MintMsg::<Extension> {
                token_id: "SEI".to_string(),
                owner: "sei_admin".to_string(),
                token_uri: None,
                extension: None,
            }),
            &[],
        )
        .unwrap();

    router
        .execute_contract(
            Addr::unchecked("admin"),
            nft_contract.clone(),
            &Cw721ExecuteMsg::<Extension, Extension>::Mint(MintMsg::<Extension> {
                token_id: "TEST".to_string(),
                owner: "test_admin".to_string(),
                token_uri: None,
                extension: None,
            }),
            &[],
        )
        .unwrap();

    nft_contract
}

fn mint_gaming_tokens_for_users(router: &mut App, roulette_address: &Addr) -> StdResult<()> {
    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "admin".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(10000),
            }],
        }))
        .unwrap();

    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "user1".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(10000),
            }],
        }))
        .unwrap();

    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "user2".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(10000),
            }],
        }))
        .unwrap();

    router
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: "sei_admin".to_string(),
            amount: vec![Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(10000),
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
            max_bet: Uint128::new(100000),
            min_bet: Uint128::new(100),
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
            max_bet: Uint128::new(100000),
            min_bet: Uint128::new(100),
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
    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);

    let new_config = Config {
        admin: Addr::unchecked("new_admin"),
        nft_contract: Addr::unchecked("nft_contract"),
        next_round_seconds: 120,
        distributor: Addr::unchecked("distributor"),
        platform_fee: Decimal::from_ratio(40 as u128, 100 as u128),
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
            next_round_seconds: 120,
            distributor: Addr::unchecked("distributor"),
            platform_fee: Decimal::from_ratio(40 as u128, 100 as u128),
        }
    );
}

#[test]
fn test_add_room() {
    let mut router = mock_app();
    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);

    let msg = ExecuteMsg::AddRoom {
        room_info: RoomConfig {
            room_name: "SEI".to_string(),
            game_denom: AssetInfo::NativeToken {
                denom: "usei".to_string(),
            },
            nft_id: "SEI".to_string(),
            max_bet: Uint128::new(100000),
            min_bet: Uint128::new(100),
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

    let _room_info: RoomInfoResponse = router
        .wrap()
        .query_wasm_smart(roulette_address.clone(), &QueryMsg::GetRoom { room_id: 1 })
        .unwrap();

    let _rooms_info: RoomsInfoResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address,
            &&QueryMsg::GetRooms {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
}

#[test]
fn test_bet_with_native_token() {
    let mut router = mock_app();
    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 1,
        bet_info: vec![BetConfig {
            direction: Direction::FirstHalf,
            amount: Uint128::new(200),
        }],
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

    let _token_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            token_address,
            &Cw20QueryMsg::Balance {
                address: roulette_address.to_string(),
            },
        )
        .unwrap();

    let _room_player: BetsInfoResponse = router
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
}

#[test]
fn test_close_round() {
    let mut router = mock_app();
    router.set_block(BlockInfo {
        height: 0,
        time: Timestamp::from_seconds(0),
        chain_id: "chain-1".to_string(),
    });

    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 1,
        bet_info: vec![BetConfig {
            direction: Direction::SecondOfThird,
            amount: Uint128::new(100),
        }],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: 0,
        time: Timestamp::from_seconds(100),
        chain_id: "chain-1".to_string(),
    });

    router
        .execute_contract(
            Addr::unchecked("user1"),
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
            direction: Direction::SecondOfThird,
            amount: Uint128::new(200),
        }],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[],
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: 0,
        time: Timestamp::from_seconds(121),
        chain_id: "chain-1".to_string(),
    });

    let bet_info: BetsInfoResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address.clone(),
            &QueryMsg::GetGameInfoForRound {
                round_id: 0,
                start_after: Some(RoundOffset {
                    room_id: 1,
                    player: Addr::unchecked("user1"),
                }),
                limit: None,
            },
        )
        .unwrap();

    println!("bet_info {:?}", bet_info);

    let close_round_msg = ExecuteMsg::CloseRound {};
    router
        .execute_contract(
            Addr::unchecked("distributor"),
            roulette_address.clone(),
            &close_round_msg,
            &[],
        )
        .unwrap();
}

#[test]
fn test_withdraw() {
    let mut router = mock_app();
    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let bet_msg = ExecuteMsg::Bet {
        room_id: 1,
        bet_info: vec![BetConfig {
            direction: Direction::SecondOfThird,
            amount: Uint128::new(100),
        }],
    };

    router
        .execute_contract(
            Addr::unchecked("user1"),
            roulette_address.clone(),
            &bet_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    let maximum_withdrawal_sei: WithdrawResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address.clone(),
            &QueryMsg::GetMaximumWithdrawlFromRoom { room_id: 1 },
        )
        .unwrap();

    let withdraw_msg = ExecuteMsg::WithdrawFromPool {
        room_id: 1,
        amount: Uint128::new(9000),
    };

    router
        .execute_contract(
            Addr::unchecked("sei_admin"),
            roulette_address.clone(),
            &withdraw_msg,
            &[],
        )
        .unwrap();

    let winner_list: WinnerListResponse = router
        .wrap()
        .query_wasm_smart(
            roulette_address.clone(),
            &QueryMsg::GetWinnerLists {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    println!("winner_list, {:?}", winner_list)
}

#[test]
fn test_deposit() {
    let mut router = mock_app();
    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let deposit_msg = ExecuteMsg::Deposit {
        room_id: 1,
        amount: Uint128::new(100),
    };
    router
        .execute_contract(
            Addr::unchecked("sei_admin"),
            roulette_address.clone(),
            &deposit_msg,
            &[Coin {
                denom: "usei".to_string(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    let native_token_balance = router
        .wrap()
        .query_balance(roulette_address.clone(), "usei".to_string())
        .unwrap();

    println!("native_token_balance {:?}", native_token_balance);

    let deposit_msg = ExecuteMsg::Deposit {
        room_id: 2,
        amount: Uint128::new(200),
    };

    router
        .execute_contract(
            Addr::unchecked("test_admin"),
            token_address.clone(),
            &Cw20ExecuteMsg::IncreaseAllowance {
                spender: roulette_address.to_string(),
                amount: Uint128::new(200),
                expires: None,
            },
            &[],
        )
        .unwrap();

    router
        .execute_contract(
            Addr::unchecked("test_admin"),
            roulette_address.clone(),
            &deposit_msg,
            &[],
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

    println!("token balance {:?}", token_balance)
}

#[test]
fn test_change_room_config() {
    let mut router = mock_app();
    let nft_address = init_cw721_contract_and_mint(&mut router);
    let roulette_address = init_roulette_contract(&mut router, nft_address);
    let token_address = init_cw20_contract(&mut router, &roulette_address);

    init_two_rooms(&mut router, &roulette_address, &token_address).unwrap();
    mint_gaming_tokens_for_users(&mut router, &roulette_address).unwrap();

    let change_room_config_msg = ExecuteMsg::ChangeRoomConfig {
        room_id: 1,
        room_name: "SEI_ROOM".to_string(),
        nft_id: "SEI_NFT".to_string(),
    };
    router
        .execute_contract(
            Addr::unchecked("admin"),
            roulette_address.clone(),
            &change_room_config_msg,
            &[],
        )
        .unwrap();

    let room_info: RoomInfoResponse = router
        .wrap()
        .query_wasm_smart(roulette_address, &QueryMsg::GetRoom { room_id: 1 })
        .unwrap();

    println!("room_config, {:?}", room_info)
}

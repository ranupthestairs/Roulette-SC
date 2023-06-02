use crate::error::ContractError;
use crate::msg::{
    BetConfig, BetsInfoResponse, Direction, ExecuteMsg, InstantiateMsg, MigrateMsg, PointRatioInfo,
};
use crate::state::{
    bet_info_key, bet_info_storage, AssetInfo, BetInfo, Config, RoomConfig, State, CONFIG,
    MINIMUMRESERVE, ROOMS, ROUND_START_SECOND, STATE, WINNERNUMBER,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    Event, MessageInfo, Order, QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg,
    WasmQuery,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw721::{AllNftInfoResponse, Cw721QueryMsg};

use crate::rand::{sha_256, Prng};

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;

const CONTRACT_NAME: &str = "Cosmos-first-roulette-gaming";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAXIMUM_SELECT: usize = 19;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    /* Validate addresses */
    CONFIG.save(deps.storage, &msg.config)?;
    STATE.save(
        deps.storage,
        &(State {
            living_round: 0,
            is_haulted: false,
            room_id: 0,
        }),
    )?;
    MINIMUMRESERVE.save(deps.storage, &Uint128::new(0))?;
    Ok(Response::new().add_attribute("action", "init_contract"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let version = cw2::get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type"));
    }
    if version.version != CONTRACT_VERSION {
        return Err(StdError::generic_err("Can only upgrade from same type"));
    }

    Ok(Response::default().add_attribute("action", "migrate_contract"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, info, config),
        ExecuteMsg::AddRoom { room_info } => execute_add_room(deps, info, room_info),
        ExecuteMsg::Bet { room_id, bet_info } => execute_bet(deps, env, info, room_id, bet_info),
        ExecuteMsg::CloseRound {} => execute_close(deps, env, info),
        ExecuteMsg::WithdrawFromPool { room_id, amount } => {
            execute_withdraw_from_pool(deps, env, info, room_id, amount)
        }
        ExecuteMsg::Deposit { room_id, amount } => {
            execute_deposit(deps, env, info, room_id, amount)
        }
        ExecuteMsg::ChangeRoomConfig {
            room_id,
            room_name,
            nft_id,
        } => execute_change_room_config(deps, info, room_id, room_name, nft_id),
        ExecuteMsg::UpdateBetLimit {
            room_id,
            max_bet,
            min_bet,
        } => execute_update_bet_limit(deps, info, room_id, max_bet, min_bet),
    }
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
) -> Result<Response, ContractError> {
    assert_is_admin(deps.as_ref(), info)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_add_room(
    deps: DepsMut,
    info: MessageInfo,
    room_info: RoomConfig,
) -> Result<Response, ContractError> {
    assert_is_admin(deps.as_ref(), info)?;

    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    STATE.update(deps.storage, |mut state| -> StdResult<_> {
        state.room_id = state.room_id + 1;
        Ok(state)
    })?;

    let _nft_info: AllNftInfoResponse<Option<Empty>> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.nft_contract.to_string(),
            msg: to_binary(&Cw721QueryMsg::AllNftInfo {
                token_id: room_info.clone().nft_id,
                include_expired: None,
            })?,
        }))?;

    //increase the room id by one
    let new_room_id = state.room_id + 1;

    //add new room.
    ROOMS.save(deps.storage, &new_room_id.to_string(), &room_info)?;

    Ok(Response::new().add_attribute("action", "add_room"))
}

fn execute_bet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    bet_info: Vec<BetConfig>,
) -> Result<Response, ContractError> {
    let player = info.sender;
    let crr_time = env.block.time.seconds();
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let living_round = state.living_round;
    //validate if this room is avaialble.

    let round_start_time = ROUND_START_SECOND.may_load(deps.storage, &living_round.to_string())?;
    match round_start_time {
        None => {}
        Some(round_start_time) => {
            if crr_time - round_start_time > config.next_round_seconds {
                return Err(ContractError::RoundFinished {});
            }
        }
    }

    //we can close the round after the first bet
    let round_start_second =
        ROUND_START_SECOND.may_load(deps.storage, &living_round.to_string())?;

    if round_start_second.is_none() {
        ROUND_START_SECOND.save(deps.storage, &living_round.to_string(), &crr_time)?;
    }

    let room_info = ROOMS.load(deps.storage, &room_id.to_string())?;
    let contract_address = env.contract.address;

    //for the token transfer
    let mut total_bet_amount = Uint128::zero();
    let mut total_point = 0;

    let mut bet_info_attributes: Vec<Attribute> = Vec::new();
    for bet in &bet_info {
        bet_info_attributes.push(attr("amount", bet.amount));
        bet_info_attributes.push(attr("direction", bet.direction.clone()));
        total_bet_amount = total_bet_amount + bet.amount;
        let point_info = get_points_ratio_information(&bet.direction)?;
        total_point = total_point + point_info.points.len();
    }

    if total_point > MAXIMUM_SELECT {
        return Err(ContractError::ExceedBetPoints {});
    }

    //check if this game is haulted or not
    assert_not_haulted(deps.as_ref())?;
    //check the min and maximum limit for game bit
    assert_min_max_limit(total_bet_amount, &room_info)?;
    //user can only bet once on round for the same room
    assert_not_double_bet(deps.as_ref(), room_id, living_round, &player)?;
    //validate the input amount for the case the input denom is native token
    validate_input_amount(&info.funds, total_bet_amount, &room_info.game_denom)?;
    //check if the user's maximum reward can exceed on the pool limit
    let withdraw_limit_for_admin = validate_maximum_reward_exceed(
        deps.as_ref(),
        &contract_address,
        room_id,
        living_round,
        &bet_info,
        &room_info.game_denom,
    )?;

    MINIMUMRESERVE.save(deps.storage, &withdraw_limit_for_admin)?;

    let bet_info_key = bet_info_key(room_id, state.living_round, &player);
    //save user bet info
    bet_info_storage().save(
        deps.storage,
        bet_info_key,
        &BetInfo {
            player: player.to_string(),
            round_id: state.living_round.to_string(),
            room_id: room_id.to_string(),
            bet_info: bet_info.clone(),
            bet_time: crr_time,
        },
    )?;

    match room_info.game_denom.clone() {
        AssetInfo::Token { contract_addr } => {
            let cw20_transfer_from_msg = get_cw20_transfer_from_msg(
                &contract_addr,
                &player,
                &contract_address,
                total_bet_amount,
            )?;

            Ok(Response::new()
                .add_attributes(vec![
                    attr("action", "bet"),
                    attr("room_id", room_id.to_string()),
                ])
                .add_attributes(bet_info_attributes)
                .add_message(cw20_transfer_from_msg))
        }
        AssetInfo::NativeToken { denom: _denom } => Ok(Response::new()
            .add_attributes(vec![
                attr("action", "bet"),
                attr("room_id", room_id.to_string()),
            ])
            .add_attributes(bet_info_attributes)),
    }
}

fn execute_close(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let crr_time = env.block.time.seconds();
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let living_round = state.living_round;

    let round_start_time = ROUND_START_SECOND.may_load(deps.storage, &living_round.to_string())?;
    match round_start_time {
        None => return Err(ContractError::RoundNotStarted {}),
        Some(round_start_time) => {
            if crr_time - round_start_time < config.next_round_seconds {
                return Err(ContractError::RoundNotFinished {});
            }
        }
    }

    let winner = rand_generator(&info, &env);

    assert_is_distributor(deps.as_ref(), info)?;

    WINNERNUMBER.save(deps.storage, &living_round.to_string(), &winner)?;
    //Update the living round
    STATE.update(deps.storage, |mut state| -> StdResult<_> {
        state.living_round = state.living_round + 1;
        Ok(state)
    })?;

    let transfer_messages: Vec<CosmosMsg> =
        distribute_reward_to_users(deps.as_ref(), living_round, state.room_id, winner)?;
    MINIMUMRESERVE.save(deps.storage, &Uint128::new(0))?;

    Ok(Response::new()
        .add_attribute("action", "close_round")
        .add_attribute("winner", winner.to_string())
        .add_attribute("round_id", living_round.to_string())
        .add_messages(transfer_messages))
}

fn distribute_reward_to_users(
    deps: Deps,
    round_id: u64,
    last_room_id: u64,
    winner: u32,
) -> StdResult<Vec<CosmosMsg>> {
    let mut transfer_msgs: Vec<CosmosMsg> = Vec::new();

    let config = CONFIG.load(deps.storage)?;
    //on a room basis, we will calculate the reward because the bet denom is different from each room.
    for i in 1..last_room_id + 1 {
        let room_id = i;
        let room_info = ROOMS.load(deps.storage, &room_id.to_string())?;
        let mut total_bet_amount = Uint128::zero();
        let mut user_winning_amount = Uint128::zero();
        //get player list for this room and this round_id
        let players_info = query_all_members_one_round_room(deps, room_id, round_id)?;
        for player_info in players_info.bets_info {
            //for each users, he can do several bets for one transaction
            for bet in &player_info.bet_info {
                total_bet_amount = total_bet_amount + bet.amount;
                let point_ratio_info = get_points_ratio_information(&bet.direction)?;
                let index = point_ratio_info.points.iter().position(|&x| x == winner);
                if index.is_some() {
                    let reward = bet.amount * Uint128::new(point_ratio_info.ratio as u128);
                    user_winning_amount = user_winning_amount + reward;
                    let recipient = &deps.api.addr_validate(&player_info.player)?;
                    let transfer_msg = match &room_info.game_denom {
                        AssetInfo::Token { contract_addr } => {
                            get_cw20_transfer_msg(contract_addr, recipient, reward)?
                        }
                        AssetInfo::NativeToken { denom } => {
                            get_bank_transfer_to_msg(recipient, denom, reward)?
                        }
                    };
                    transfer_msgs.push(transfer_msg);
                }
            }
        }

        //send some percent of round reward to the admin as platform fee.
        if total_bet_amount > user_winning_amount {
            let reward_for_admin_side = total_bet_amount - user_winning_amount;
            let game_fee = reward_for_admin_side * config.platform_fee;
            if game_fee > Uint128::zero() {
                let transfer_msg = match &room_info.game_denom {
                    AssetInfo::Token { contract_addr } => {
                        get_cw20_transfer_msg(contract_addr, &config.admin, game_fee)?
                    }
                    AssetInfo::NativeToken { denom } => {
                        get_bank_transfer_to_msg(&config.admin, denom, game_fee)?
                    }
                };
                transfer_msgs.push(transfer_msg);
            }
        }
    }
    Ok(transfer_msgs)
}

fn execute_withdraw_from_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let room_info = validate_room_id(deps.as_ref(), room_id)?;
    let contract_address = env.contract.address;
    assert_is_room_owner(deps.as_ref(), &info, &room_info)?;
    let withdrawal_amount = get_withdrawal_amount(deps.as_ref(), &room_info, &contract_address)?;
    if withdrawal_amount < amount {
        return Err(ContractError::WithdrawalMoneyExceeded {
            withdrawal_amount,
            amount,
        });
    }

    let transfer_msg = match room_info.game_denom {
        AssetInfo::Token { contract_addr } => {
            get_cw20_transfer_msg(&contract_addr, &info.sender, amount)?
        }
        AssetInfo::NativeToken { denom } => get_bank_transfer_to_msg(&info.sender, &denom, amount)?,
    };

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("withdrawer", info.sender.to_string())
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("amount", amount))
}

fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let room_info = validate_room_id(deps.as_ref(), room_id)?;
    let game_denom = room_info.clone().game_denom;
    let contract_address = env.contract.address;

    assert_is_room_owner(deps.as_ref(), &info, &room_info)?;
    validate_input_amount(&info.funds, amount, &game_denom)?;

    let player = info.sender;

    match room_info.game_denom {
        AssetInfo::Token { contract_addr } => {
            let cw20_transfer_from_msg =
                get_cw20_transfer_from_msg(&contract_addr, &player, &contract_address, amount)?;

            Ok(Response::new()
                .add_attributes(vec![
                    attr("action", "deposit"),
                    attr("room_id", room_id.to_string()),
                    attr("amount", amount.to_string()),
                ])
                .add_message(cw20_transfer_from_msg))
        }
        AssetInfo::NativeToken { denom: _denom } => Ok(Response::new().add_attributes(vec![
            attr("action", "deposit"),
            attr("room_id", room_id.to_string()),
            attr("amount", amount.to_string()),
        ])),
    }
}

fn execute_change_room_config(
    deps: DepsMut,
    info: MessageInfo,
    room_id: u64,
    room_name: String,
    nft_id: String,
) -> Result<Response, ContractError> {
    assert_is_admin(deps.as_ref(), info)?;
    validate_room_id(deps.as_ref(), room_id)?;

    ROOMS.update(
        deps.storage,
        &room_id.to_string(),
        |room_info| -> StdResult<_> {
            let mut room_info = room_info.unwrap();
            room_info.room_name = room_name.clone();
            room_info.nft_id = nft_id.clone();
            Ok(room_info)
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "room_update")
        .add_attribute("room_name", room_name)
        .add_attribute("nft_id", nft_id))
}

fn execute_update_bet_limit(
    deps: DepsMut,
    info: MessageInfo,
    room_id: u64,
    max_bet: Uint128,
    min_bet: Uint128,
) -> Result<Response, ContractError> {
    let room_info = validate_room_id(deps.as_ref(), room_id)?;

    assert_is_room_owner(deps.as_ref(), &info, &room_info)?;

    ROOMS.update(
        deps.storage,
        &room_id.to_string(),
        |room_info| -> StdResult<_> {
            let mut room_info = room_info.unwrap();
            room_info.max_bet = max_bet;
            room_info.min_bet = min_bet;
            Ok(room_info)
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "room_update")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("max_bet", max_bet)
        .add_attribute("min_bet", min_bet))
}

fn assert_not_haulted(deps: Deps) -> StdResult<bool> {
    let state = STATE.load(deps.storage)?;
    let is_haulted = state.is_haulted;
    if is_haulted {
        return Err(StdError::generic_err("Contract is haulted"));
    }
    Ok(true)
}

fn assert_is_admin(deps: Deps, info: MessageInfo) -> StdResult<bool> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(StdError::generic_err(format!(
            "Only the admin can execute this function. Admin: {}, Sender: {}",
            config.admin, info.sender
        )));
    }

    Ok(true)
}

fn assert_is_room_owner(deps: Deps, info: &MessageInfo, room: &RoomConfig) -> StdResult<bool> {
    let config = CONFIG.load(deps.storage)?;

    let nft_contract = config.nft_contract.to_string();
    let nft_id = room.nft_id.clone();
    //room owner is considerd as NFT, so will check the owner of this NFT

    let nft_info: AllNftInfoResponse<Option<Empty>> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_contract,
            msg: to_binary(&Cw721QueryMsg::AllNftInfo {
                token_id: nft_id,
                include_expired: None,
            })?,
        }))?;

    let room_owner = nft_info.access.owner;

    if info.sender.to_string() != room_owner {
        return Err(StdError::generic_err(format!(
            "Only the admin of room can execute this function. Room Admin: {}, Sender: {}",
            room_owner, info.sender
        )));
    }

    Ok(true)
}

fn assert_is_distributor(deps: Deps, info: MessageInfo) -> StdResult<bool> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.distributor {
        return Err(StdError::generic_err(format!(
            "Only the distributor can execute this function. Distributor: {}, Sender: {}",
            config.distributor, info.sender
        )));
    }

    Ok(true)
}

fn assert_min_max_limit(total_bet_amount: Uint128, room_info: &RoomConfig) -> StdResult<bool> {
    if total_bet_amount < room_info.min_bet || total_bet_amount > room_info.max_bet {
        return Err(StdError::GenericErr {
            msg: format!(
                "You must bet with the amount between {}  and {}",
                room_info.min_bet, room_info.max_bet
            ),
        });
    }

    Ok(true)
}

fn assert_not_double_bet(
    deps: Deps,
    room_id: u64,
    living_round: u64,
    player: &Addr,
) -> StdResult<bool> {
    let bet_info_key = bet_info_key(room_id, living_round, &player);
    let bet_info = bet_info_storage().may_load(deps.storage, bet_info_key)?;
    if bet_info.is_some() {
        return Err(StdError::GenericErr {
            msg: format!("This user already bet on this round for this room"),
        });
    }

    Ok(true)
}

fn validate_input_amount(
    actual_funds: &[Coin],
    amount: Uint128,
    game_denom: &AssetInfo,
) -> Result<(), ContractError> {
    match game_denom {
        AssetInfo::Token {
            contract_addr: _contract_address,
        } => Ok(()),
        AssetInfo::NativeToken { denom } => {
            let actual = get_amount_for_denom(actual_funds, &denom);
            if actual.amount != amount {
                return Err(ContractError::InsufficientFunds {});
            }
            if &actual.denom != denom {
                return Err(ContractError::IncorrectNativeDenom {
                    provided: actual.denom,
                    required: denom.to_string(),
                });
            }
            Ok(())
        }
    }
}

pub fn get_withdrawal_amount(
    deps: Deps,
    room_info: &RoomConfig,
    recipient: &Addr,
) -> StdResult<Uint128> {
    let room_denom = room_info.game_denom.clone();
    let balance = match room_denom {
        AssetInfo::Token { contract_addr } => {
            get_cw20_token_amount(deps, &contract_addr, recipient)?
        }
        AssetInfo::NativeToken { denom } => get_native_token_amount(deps, &denom, recipient)?,
    };

    let minimum_limit_for_pool = MINIMUMRESERVE.load(deps.storage)?;
    //admin can not withdraw money which is really exceeded than user's limit
    let maximum_withdrawal = balance - minimum_limit_for_pool;

    Ok(maximum_withdrawal)
}

fn validate_room_id(deps: Deps, room_id: u64) -> StdResult<RoomConfig> {
    let room = ROOMS.may_load(deps.storage, &room_id.to_string())?;
    if room.is_none() {
        return Err(StdError::generic_err(format!("This room does not exist")));
    }
    Ok(room.unwrap())
}

fn validate_maximum_reward_exceed(
    deps: Deps,
    contract_address: &Addr,
    room_id: u64,
    round_id: u64,
    bet_info: &Vec<BetConfig>,
    denom: &AssetInfo,
) -> StdResult<Uint128> {
    let room_round_players = query_all_members_one_round_room(deps, room_id, round_id)?;
    //token or native token amount of this contract
    let token_hold_amount = match denom {
        AssetInfo::NativeToken { denom } => get_native_token_amount(deps, denom, contract_address)?,
        AssetInfo::Token { contract_addr } => {
            get_cw20_token_amount(deps, contract_addr, contract_address)?
        }
    };

    //this parameter must be needed so that the admin can not exceed for the maximum reward for users.
    let mut minimum_reserve_limit = MINIMUMRESERVE.load(deps.storage)?;

    //we will check for each point
    for i in 0..38 {
        let point = i as u32;
        //this number will be the output of reward at the poing of point(above number).
        let mut maximum_amount_test = Uint128::zero();
        //first get all player lists for one room and one round_id, users can not bet twice on one round, one room
        //every user can do several bets with vector type for one transaction
        //get available points for each bet config and ratio
        //check if this includes the point and if it includes we will increase the amount
        //This step is for prev users for this room and id

        for player_bet_info in &room_round_players.bets_info {
            for bet in &player_bet_info.bet_info {
                let point_ratio_info = get_points_ratio_information(&bet.direction)?;
                let index = point_ratio_info.points.iter().position(|&x| x == point);
                if index.is_some() {
                    maximum_amount_test = maximum_amount_test
                        + bet.amount * Uint128::new(point_ratio_info.ratio as u128);
                }
            }
        }

        //add the additional info for this user(new comer) and this is the step for the current user join
        for bet in bet_info {
            let point_ratio_info = get_points_ratio_information(&bet.direction)?;
            let index = point_ratio_info.points.iter().position(|&x| x == point);
            if index.is_some() {
                maximum_amount_test =
                    maximum_amount_test + bet.amount * Uint128::new(point_ratio_info.ratio as u128);
            }
        }

        if maximum_amount_test > minimum_reserve_limit {
            minimum_reserve_limit = maximum_amount_test;
        }

        if maximum_amount_test > token_hold_amount {
            return Err(StdError::GenericErr {
                msg: format!(
                    "The contract will have {} of tokens after this bet, but if {} is selected as winner, the maximum reward will be {}",
                    token_hold_amount,
                    point,
                    maximum_amount_test
                ),
            });
        }
    }

    Ok(minimum_reserve_limit)
}

pub fn get_points_ratio_information(direction: &Direction) -> StdResult<PointRatioInfo> {
    match direction {
        Direction::Odd => Ok(PointRatioInfo {
            points: vec![
                1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31, 33, 35,
            ],
            ratio: 2,
        }),
        Direction::Even => Ok(PointRatioInfo {
            points: vec![
                2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36,
            ],
            ratio: 2,
        }),
        Direction::FirstHalf => Ok(PointRatioInfo {
            points: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
            ],
            ratio: 2,
        }),
        Direction::SecondHalf => Ok(PointRatioInfo {
            points: vec![
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
            ],
            ratio: 2,
        }),
        Direction::Blue => Ok(PointRatioInfo {
            points: vec![
                1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36,
            ],
            ratio: 2,
        }),
        Direction::Black => Ok(PointRatioInfo {
            points: vec![
                2, 4, 6, 8, 10, 11, 13, 15, 17, 20, 22, 24, 26, 28, 29, 31, 33, 35,
            ],
            ratio: 2,
        }),
        Direction::Row { id } => {
            if *id < 1 || *id > 3 {
                return Err(StdError::GenericErr {
                    msg: format!("The row select parameter must be one of 1,2 and 3"),
                });
            }
            let mut row_ids: Vec<u32> = Vec::new();
            for i in 0..12 {
                row_ids.push(id + i * 3);
            }
            Ok(PointRatioInfo {
                points: row_ids,
                ratio: 3,
            })
        }
        Direction::Column { id } => {
            if *id < 1 || *id > 12 {
                return Err(StdError::GenericErr {
                    msg: format!("The row select parameter must be in tje range pf 1 to 12 "),
                });
            }
            let id_start = (id - 1) * 3 + 1;
            Ok(PointRatioInfo {
                points: vec![id_start, id_start + 1, id_start + 2],
                ratio: 12,
            })
        }
        Direction::FirstOfThird => Ok(PointRatioInfo {
            points: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            ratio: 3,
        }),
        Direction::SecondOfThird => Ok(PointRatioInfo {
            points: vec![13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24],
            ratio: 3,
        }),
        Direction::ThirdOfThird => Ok(PointRatioInfo {
            points: vec![25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36],
            ratio: 3,
        }),
        Direction::Single { id } => {
            if *id < 1 || *id > 36 {
                return Err(StdError::GenericErr {
                    msg: format!("The row select parameter must be in tje range pf 1 to 36 "),
                });
            }
            Ok(PointRatioInfo {
                points: vec![*id],
                ratio: 36,
            })
        }
        Direction::Zero {} => Ok(PointRatioInfo {
            points: vec![0],
            ratio: 36,
        }),
        Direction::ZeroZero {} => Ok(PointRatioInfo {
            points: vec![37],
            ratio: 36,
        }),
    }
}

pub fn new_entropy(info: &MessageInfo, env: &Env, seed: &[u8], entropy: &[u8]) -> [u8; 32] {
    // 16 here represents the lengths in bytes of the block height and time.
    let entropy_len = 16 + info.sender.to_string().len() + entropy.len();
    let mut rng_entropy = Vec::with_capacity(entropy_len);
    rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
    rng_entropy.extend_from_slice(&env.block.time.nanos().to_be_bytes());
    rng_entropy.extend_from_slice(&info.sender.as_bytes());
    rng_entropy.extend_from_slice(entropy);

    let mut rng = Prng::new(seed, &rng_entropy);

    rng.rand_bytes()
}

pub fn rand_generator(info: &MessageInfo, env: &Env) -> u32 {
    let prng_seed: Vec<u8> = sha_256(base64::encode("entropy").as_bytes()).to_vec();
    let random_seed = new_entropy(&info, &env, prng_seed.as_ref(), prng_seed.as_ref());
    let mut rng = ChaChaRng::from_seed(random_seed);
    let rand_num = rng.next_u32();
    rand_num % 38
}

pub fn query_all_members_one_round_room(
    deps: Deps,
    room_id: u64,
    round_id: u64,
) -> StdResult<BetsInfoResponse> {
    let bets_info = bet_info_storage()
        .idx
        .room_round_players
        .prefix((room_id.to_string(), round_id.to_string()))
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(BetsInfoResponse { bets_info })
}

fn get_amount_for_denom(coins: &[Coin], denom: &str) -> Coin {
    let amount: Uint128 = coins
        .iter()
        .filter(|c| c.denom == denom)
        .map(|c| c.amount)
        .sum();
    Coin {
        amount,
        denom: denom.to_string(),
    }
}

pub fn get_cw20_transfer_msg(
    token_addr: &Addr,
    recipient: &Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let transfer_cw20_msg = Cw20ExecuteMsg::Transfer {
        recipient: recipient.into(),
        amount,
    };

    let exec_cw20_transfer_msg = WasmMsg::Execute {
        contract_addr: token_addr.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };

    let cw20_transfer_msg: CosmosMsg = exec_cw20_transfer_msg.into();
    Ok(cw20_transfer_msg)
}

pub fn get_cw20_transfer_from_msg(
    token_addr: &Addr,
    owner: &Addr,
    recipient: &Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
        owner: owner.into(),
        recipient: recipient.into(),
        amount,
    };

    let exec_cw20_transfer_msg = WasmMsg::Execute {
        contract_addr: token_addr.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };

    let cw20_transfer_msg: CosmosMsg = exec_cw20_transfer_msg.into();
    Ok(cw20_transfer_msg)
}

pub fn get_cw20_burn_from_msg(
    token_addr: &Addr,
    owner: &Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let burn_cw20_msg = Cw20ExecuteMsg::BurnFrom {
        owner: owner.into(),
        amount,
    };
    let exec_cw20_burn_msg = WasmMsg::Execute {
        contract_addr: token_addr.into(),
        msg: to_binary(&burn_cw20_msg)?,
        funds: vec![],
    };

    let cw20_burn_msg: CosmosMsg = exec_cw20_burn_msg.into();
    Ok(cw20_burn_msg)
}

pub fn get_bank_transfer_to_msg(
    recipient: &Addr,
    denom: &str,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let transfer_bank_msg = cosmwasm_std::BankMsg::Send {
        to_address: recipient.into(),
        amount: vec![Coin {
            denom: denom.to_string(),
            amount,
        }],
    };

    let transfer_bank_cosmos_msg: CosmosMsg = transfer_bank_msg.into();
    Ok(transfer_bank_cosmos_msg)
}

fn get_native_token_amount(deps: Deps, denom: &String, recipient: &Addr) -> StdResult<Uint128> {
    let balance = deps.querier.query_balance(recipient, denom)?;
    Ok(balance.amount)
}

fn get_cw20_token_amount(deps: Deps, contract_addr: &Addr, recipient: &Addr) -> StdResult<Uint128> {
    let balance: BalanceResponse = deps.querier.query_wasm_smart(
        contract_addr.to_string(),
        &(Cw20QueryMsg::Balance {
            address: recipient.to_string(),
        }),
    )?;
    Ok(balance.balance)
}

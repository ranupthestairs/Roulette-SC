use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::execute::get_withdrawal_amount;
use crate::msg::{
    BetsInfoResponse, ConfigResponse, QueryMsg, RoomInfoResponse, RoomsInfoResponse, StateResponse,
    Winner, WinnerListResponse, WinnerResponse, WithdrawResponse,
};
use crate::state::{bet_info_key, bet_info_storage, CONFIG, ROOMS, STATE, WINNERNUMBER};

const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::GetRoom { room_id } => to_binary(&query_room_info(deps, room_id)?),
        QueryMsg::GetRooms { start_after, limit } => {
            to_binary(&query_get_rooms(deps, start_after, limit)?)
        }
        QueryMsg::GetPlayersForOneRoundOneRoom {
            room_id,
            round_id,
            start_after,
            limit,
        } => to_binary(&query_get_players_for_one_round_one_room(
            deps,
            room_id,
            round_id,
            start_after,
            limit,
        )?),
        QueryMsg::GetPlayerInfosForRoom {
            room_id,
            player,
            start_after,
            limit,
        } => to_binary(&query_player_infos_for_room(
            deps,
            room_id,
            player,
            start_after,
            limit,
        )?),
        QueryMsg::GetMaximumWithdrawlFromRoom { room_id } => {
            to_binary(&query_maximum_withdrwal(deps, env, room_id)?)
        }
        QueryMsg::GetWinnerRound { round_id } => to_binary(&query_winner_round(deps, round_id)?),
        QueryMsg::GetWinnerLists { start_after, limit } => {
            to_binary(&query_get_round_lists(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}

fn query_room_info(deps: Deps, room_id: u64) -> StdResult<RoomInfoResponse> {
    let room = ROOMS.load(deps.storage, &room_id.to_string())?;
    Ok(RoomInfoResponse { room })
}

fn query_get_rooms(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<RoomsInfoResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.to_string().into()));

    let rooms = ROOMS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(RoomsInfoResponse { rooms })
}

fn query_get_players_for_one_round_one_room(
    deps: Deps,
    room_id: u64,
    round_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BetsInfoResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = if let Some(start) = start_after {
        Some(Bound::exclusive(bet_info_key(
            room_id,
            round_id,
            &deps.api.addr_validate(&start)?,
        )))
    } else {
        None
    };

    let bets_info = bet_info_storage()
        .idx
        .room_round_players
        .prefix((room_id.to_string(), round_id.to_string()))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BetsInfoResponse { bets_info })
}

fn query_player_infos_for_room(
    deps: Deps,
    room_id: u64,
    player: Addr,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<BetsInfoResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = if let Some(start) = start_after {
        Some(Bound::exclusive(bet_info_key(room_id, start, &player)))
    } else {
        None
    };

    let bets_info = bet_info_storage()
        .idx
        .room_player
        .prefix((room_id.to_string(), player.to_string()))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BetsInfoResponse { bets_info })
}

fn query_maximum_withdrwal(deps: Deps, env: Env, room_id: u64) -> StdResult<WithdrawResponse> {
    let room = ROOMS.may_load(deps.storage, &room_id.to_string())?;
    match room {
        Some(room_info) => {
            let contract_address = env.contract.address;
            let withdrawal_amount = get_withdrawal_amount(deps, &room_info, &contract_address)?;
            Ok(WithdrawResponse {
                amount: withdrawal_amount,
            })
        }
        None => Ok(WithdrawResponse {
            amount: Uint128::zero(),
        }),
    }
}

fn query_winner_round(deps: Deps, round_id: u64) -> StdResult<WinnerResponse> {
    let winner = WINNERNUMBER.may_load(deps.storage, &round_id.to_string())?;
    match winner {
        Some(winner) => Ok(WinnerResponse {
            winner: Winner {
                winner,
                round_id: round_id.to_string(),
            },
        }),
        None => Ok(WinnerResponse {
            winner: Winner {
                winner: 40,
                round_id: round_id.to_string(),
            },
        }),
    }
}

fn query_get_round_lists(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<WinnerListResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.to_string().into()));

    let winner_list = WINNERNUMBER
        .range(deps.storage, start, None, Order::Descending)
        .take(limit)
        .map(|res| {
            res.map(|item| Winner {
                round_id: item.0,
                winner: item.1,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(WinnerListResponse { winner_list })
}

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use crate::state::{BetInfo, Config, RoomConfig, RoomInfo, State};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        config: Config,
    },
    AddRoom {
        room_info: RoomConfig,
    },
    Bet {
        room_id: u64,
        bet_info: Vec<BetConfig>,
    },
    CloseRound {},
    WithdrawFromPool {
        room_id: u64,
        amount: Uint128,
    },
    Deposit {
        room_id: u64,
        amount: Uint128,
    },
    ChangeRoomConfig {
        room_id: u64,
        room_name: String,
        nft_id: String,
    },
    UpdateBetLimit {
        room_id: u64,
        max_bet: Uint128,
        min_bet: Uint128,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    State {},
    AllState {},
    GetRoom {
        room_id: u64,
    },
    GetRooms {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    GetPlayersForOneRoundOneRoom {
        room_id: u64,
        round_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetPlayerInfosForRoom {
        room_id: u64,
        player: Addr,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    GetMaximumWithdrawlFromRoom {
        room_id: u64,
    },
    GetWinnerRound {
        round_id: u64,
    },
    GetWinnerLists {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    GetGameInfoForRound {
        round_id: u64,
        start_after: Option<RoundOffset>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct RoundOffset {
    pub room_id: u64,
    pub player: Addr,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct StateResponse {
    pub state: State,
}

#[cw_serde]
pub struct AllStateResponse {
    pub state: State,
    pub config: Config,
    pub crr_time: u64,
    pub round_start_second: u64,
}

#[cw_serde]
pub struct RoomInfoResponse {
    pub room: RoomInfo,
}

#[cw_serde]
pub struct RoomsInfoResponse {
    pub rooms: Vec<RoomInfo>,
}

#[cw_serde]
pub struct BetsInfoResponse {
    pub bets_info: Vec<BetInfo>,
}

#[cw_serde]
pub struct WithdrawResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct Winner {
    pub winner: u32,
    pub round_id: String,
}

#[cw_serde]
pub struct WinnerResponse {
    pub winner: Winner,
}

#[cw_serde]
pub struct WinnerListResponse {
    pub winner_list: Vec<Winner>,
}

#[cw_serde]
pub enum Direction {
    Odd,
    Even,
    FirstHalf,
    SecondHalf,
    Blue,
    Black,
    Row { id: u32 },
    Column { id: u32 },
    FirstOfThird,
    SecondOfThird,
    ThirdOfThird,
    Single { id: u32 },
    Zero {},
    ZeroZero {},
}

#[cw_serde]
pub struct BetConfig {
    pub direction: Direction,
    pub amount: Uint128,
}

#[cw_serde]
pub struct PointRatioInfo {
    pub points: Vec<u32>,
    pub ratio: u32,
}

impl Into<std::string::String> for Direction {
    fn into(self) -> std::string::String {
        // Convert the direction to a string and return it
        match self {
            Direction::Odd => "odd".to_string(),
            Direction::Even => "even".to_string(),
            Direction::FirstHalf => "first_half".to_string(),
            Direction::SecondHalf => "second_half".to_string(),
            Direction::Blue => "red".to_string(),
            Direction::Black => "black".to_string(),
            Direction::Row { id: _ } => "row".to_string(),
            Direction::Column { id: _ } => "column".to_string(),
            Direction::FirstOfThird => "first_of_third".to_string(),
            Direction::SecondOfThird => "second of third".to_string(),
            Direction::ThirdOfThird => "third of third".to_string(),
            Direction::Single { id: _ } => "single".to_string(),
            Direction::Zero {} => "zero".to_string(),
            Direction::ZeroZero {} => "zero_zero".to_string(),
        }
    }
}

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

use crate::msg::{BetConfig, Direction};

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
pub const MINIMUMRESERVE: Item<Uint128> = Item::new("minimum_reserve");
pub const ROOMS: Map<&str, RoomConfig> = Map::new("rounds");
pub const WINNERNUMBER: Map<&str, u32> = Map::new("random_winner");

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub nft_contract: Addr,
    pub minimum_bet: Uint128,
    pub maximum_bet: Uint128,
    pub next_round_seconds: u64,
    pub distributor: Addr,
}

#[cw_serde]
pub struct State {
    pub living_round: u64,
    pub is_haulted: bool,
    pub room_id: u64,
}

#[cw_serde]
pub struct RoomConfig {
    pub room_name: String,
    pub game_denom: AssetInfo,
    pub nft_id: String,
}

#[cw_serde]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

#[cw_serde]
pub struct BetInfo {
    pub player: String,
    pub round_id: String,
    pub room_id: String,
    pub bet_info: Vec<BetConfig>,
}

// /// Primary key for betinfo: (room_id, round_id, player)
pub type BetInfoKey = (String, String, String);
// /// Convenience bid key constructor
pub fn bet_info_key(room_id: u64, round_id: u64, player: &Addr) -> BetInfoKey {
    (
        room_id.to_string().clone(),
        round_id.to_string().clone(),
        player.to_string().clone(),
    )
}

// /// Defines incides for accessing bids
pub struct BetInfoIndicies<'a> {
    pub player: MultiIndex<'a, String, BetInfo, BetInfoKey>,
    pub round_id: MultiIndex<'a, String, BetInfo, BetInfoKey>,
    pub room_id: MultiIndex<'a, String, BetInfo, BetInfoKey>,
    pub room_player: MultiIndex<'a, (String, String), BetInfo, BetInfoKey>,
    pub room_round_players: MultiIndex<'a, (String, String), BetInfo, BetInfoKey>,
}

impl<'a> IndexList<BetInfo> for BetInfoIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<BetInfo>> + '_> {
        let v: Vec<&dyn Index<BetInfo>> = vec![
            &self.player,
            &self.round_id,
            &self.room_id,
            &self.room_player,
            &self.room_round_players,
        ];
        Box::new(v.into_iter())
    }
}

pub fn bet_info_storage<'a>() -> IndexedMap<'a, BetInfoKey, BetInfo, BetInfoIndicies<'a>> {
    let indexes = BetInfoIndicies {
        player: MultiIndex::new(
            |_pk: &[u8], d: &BetInfo| d.player.clone(),
            "bet_info",
            "bet_info_collection",
        ),
        round_id: MultiIndex::new(
            |_pk: &[u8], d: &BetInfo| d.round_id.clone(),
            "bet_info",
            "round_id",
        ),
        room_id: MultiIndex::new(
            |_pk: &[u8], d: &BetInfo| d.room_id.clone(),
            "bet_info",
            "room_id",
        ),
        room_player: MultiIndex::new(
            |_pk: &[u8], d: &BetInfo| (d.room_id.clone(), d.player.clone()),
            "bet_info",
            "room_player",
        ),
        room_round_players: MultiIndex::new(
            |_pk: &[u8], d: &BetInfo| (d.room_id.clone(), d.round_id.clone()),
            "bet_info",
            "round_room_players",
        ),
    };
    IndexedMap::new("bet_info", indexes)
}

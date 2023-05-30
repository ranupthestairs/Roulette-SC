use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use roulette_game::msg::{
    AllStateResponse, BetConfig, BetsInfoResponse, ConfigResponse, Direction, ExecuteMsg,
    InstantiateMsg, PointRatioInfo, QueryMsg, RoomInfoResponse, RoomsInfoResponse, StateResponse,
};
use roulette_game::state::{AssetInfo, BetInfo, Config, RoomConfig, State};

fn main() {
    let mut out_dir = current_dir().unwrap();

    // out_dir.push("contracts/price_prediction/schema");
    out_dir.push("schema");

    println!("creating dir at {:?}", out_dir.display());
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(AssetInfo), &out_dir);
    export_schema(&schema_for!(BetInfo), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(RoomConfig), &out_dir);
    export_schema(&schema_for!(State), &out_dir);
    export_schema(&schema_for!(BetConfig), &out_dir);
    export_schema(&schema_for!(BetsInfoResponse), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(Direction), &out_dir);
    export_schema(&schema_for!(PointRatioInfo), &out_dir);
    export_schema(&schema_for!(RoomInfoResponse), &out_dir);
    export_schema(&schema_for!(RoomsInfoResponse), &out_dir);
    export_schema(&schema_for!(StateResponse), &out_dir);
    export_schema(&schema_for!(AllStateResponse), &out_dir);
}

use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InsufficientFunds")]
    InsufficientFunds {},

    #[error("Incorrect native denom: provided: {provided}, required: {required}")]
    IncorrectNativeDenom { provided: String, required: String },
}

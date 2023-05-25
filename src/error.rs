use thiserror::Error;

use cosmwasm_std::{StdError, Uint128};

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

    #[error("You can withdraw ax maximum {withdrawal_amount} because of current user's betting reward for maximum case, now you are trying to withdraw {amount}")]
    WithdrawalMoneyExceeded {
        withdrawal_amount: Uint128,
        amount: Uint128,
    },
}

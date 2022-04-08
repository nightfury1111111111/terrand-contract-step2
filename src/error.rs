use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Beacon not found")]
    NoBeacon {},

    #[error("Not a valid drand signature")]
    InvalidSignature {},

    #[error("Do not send funds with {0}")]
    DoNotSendFunds(String),

    #[error("Drand round already added {0}")]
    DrandRoundAlreadyAdded(String),
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

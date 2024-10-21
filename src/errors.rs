use crate::order::Order;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum OrderError {
    AssetsNotMatching,
    TooFewSourceAssets,
    TooManyTargetAssets,
    FailedFill(Order),
    MissingId,
    InternalError(String),
}

impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderError::AssetsNotMatching => write!(f, "Assets do not match"),
            OrderError::TooFewSourceAssets => write!(f, "Too few source assets"),
            OrderError::TooManyTargetAssets => write!(f, "Too many target assets"),
            OrderError::FailedFill(order) => write!(f, "Failed to fill order: {:?}", order),
            OrderError::MissingId => write!(f, "Missing ID"),
            OrderError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

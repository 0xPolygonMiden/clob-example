use crate::order::Order;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum OrderError {
    AssetsNotMatching,
    PriceTooHigh(u64, u64),
    FailedFill(Order),
    MissingId,
    InternalError(String),
}

impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderError::AssetsNotMatching => write!(f, "Assets do not match"),
            OrderError::PriceTooHigh(incoming_order_price, existing_order_price) => write!(
                f,
                "Existing order price is {} while incoming order price is {}.",
                existing_order_price, incoming_order_price
            ),
            OrderError::FailedFill(order) => write!(f, "Failed to fill order: {:?}", order),
            OrderError::MissingId => write!(f, "Missing ID"),
            OrderError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

use crate::bigint::BigInt;

use once_cell::sync::Lazy;
use std::{
    error::Error as ErrorTrait,
    fmt::{self, Debug},
};

pub mod constraints;
pub mod rsa_hidden_order_group;
pub mod unsigned_rsa_hidden_order_group;

pub use rsa_hidden_order_group::RsaHiddenOrderGroup;
pub use unsigned_rsa_hidden_order_group::UnsignedRsaHiddenOrderGroup;

//TODO: https://github.com/rust-num/num-bigint/issues/181
pub trait RsaGroupParams: Clone + Eq + Debug + Send + Sync {
    const G: Lazy<BigInt>; // generator
    const M: Lazy<BigInt>; // modulus
}

pub trait UnsignedRsaGroupParams: Clone + Eq + Debug + Send + Sync {
    const G: Option<Lazy<BigInt>>; // generator (optional for cyclic groups)
    const M: Lazy<BigInt>; // modulus
}

#[derive(Debug)]
pub enum RsaHOGError {
    NotInvertible,
    NotCyclic,
}

impl ErrorTrait for RsaHOGError {
    fn source(self: &Self) -> Option<&(dyn ErrorTrait + 'static)> {
        None
    }
}

impl fmt::Display for RsaHOGError {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            RsaHOGError::NotInvertible => format!("Group element not invertible"),
            RsaHOGError::NotCyclic => format!("Group is not cyclic, missing generator"),
        };
        write!(f, "{}", msg)
    }
}

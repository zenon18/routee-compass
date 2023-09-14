use derive_more::{Add, Div, Mul, Neg, Sub, Sum};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display};

#[derive(
    Copy,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    PartialOrd,
    Eq,
    Hash,
    Debug,
    Default,
    Add,
    Sub,
    Mul,
    Div,
    Sum,
    Neg,
)]
pub struct Distance(pub OrderedFloat<f64>);

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Display for Distance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Distance {
    pub fn new(value: f64) -> Distance {
        Distance(OrderedFloat(value))
    }
    pub fn to_f64(&self) -> f64 {
        (self.0).0
    }
    pub const ZERO: Distance = Distance(OrderedFloat(0.0));
    pub const ONE: Distance = Distance(OrderedFloat(1.0));
}

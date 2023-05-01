#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::{cell::RefCell, collections::BinaryHeap, rc::Rc};

pub enum OrderSide {
    Buy,
    Sell,
}

pub struct Order {
    pub id: u64,
    pub volume: Decimal,
    pub price: Decimal,
    pub order_side: OrderSide,
    pub client: String,
    pub entry_time: DateTime<Utc>,
    pub event_time: DateTime<Utc>,
}

use std::collections::HashMap;
use rust_decimal::prelude::*;
use super::orderbook::{Order, OrderBook};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct TradingPair {
    base: String,
    quote: String,
}

impl TradingPair {
    pub fn new(base: String, quote: String) -> TradingPair {
        TradingPair {
            base,
            quote,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}

pub struct MatchingEngine {
    orderbooks: HashMap<TradingPair, OrderBook>,
}

impl MatchingEngine {
    pub fn new() -> MatchingEngine {
        MatchingEngine {
            orderbooks: HashMap::new(),
        }
    }

    pub fn add_new_market(&mut self, pair: TradingPair) {
        self.orderbooks.insert(pair.clone(), OrderBook::new());
        println!("Added new market: {:?}", pair);
    }

    pub fn place_limit_order(&mut self, price: Decimal, pair: TradingPair, order: Order) -> Result<(), String> {
        match self.orderbooks.get_mut(&pair) {
            Some(orderbook) => {
                orderbook.add_limit_order(order, price);
                Ok(())
            }
            None => Err(format!("No orderbook for trading pair: {:?}", pair.to_string())),
        }
    }
}
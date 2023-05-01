use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    Bid,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Order {
    pub tick_id: String,
    pub exchange_id: u64,
    pub order_type: OrderType,
    pub shares: Decimal,
    pub limit_price: Decimal,
    pub entry_time: DateTime<Utc>,
    pub event_time: DateTime<Utc>,
}

impl Order {
    pub fn new(
        tick_id: String,
        exchange_id: u64,
        order_type: OrderType,
        shares: Decimal,
        limit_price: Decimal,
        entry_time: DateTime<Utc>,
        event_time: DateTime<Utc>,
    ) -> Self {
        Self {
            tick_id,
            exchange_id,
            order_type,
            shares,
            limit_price,
            entry_time,
            event_time,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub limit_price: Decimal,
    pub orders: HashMap<u64, Order>,
    pub parent: Option<Box<Limit>>,
    pub size: Decimal,
    pub total_volume: Decimal,
    pub order_count: u64,
}

impl Limit {
    pub fn new(limit_price: Decimal) -> Self {
        Self {
            limit_price,
            orders: HashMap::new(),
            parent: None,
            size: Decimal::zero(),
            total_volume: Decimal::new(0, 0),
            order_count: 0,
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.size += order.shares;
        self.total_volume += order.shares * order.limit_price;
        self.order_count += 1;
        self.orders.insert(order.exchange_id, order);
    }

    pub fn remove_order(&mut self, order: Order) {
        if let Some(order) = self.orders.remove(&order.exchange_id) {
            self.size -= order.shares;
            self.total_volume -= order.shares * order.limit_price;
            self.order_count -= 1;
        }

        if self.parent.is_none() && self.orders.is_empty() {
            self.size = Decimal::new(0, 0);
            self.total_volume = Decimal::new(0, 0);
            self.order_count = 0;
        }

        if self.size == Decimal::new(0, 0) {
            if let Some(parent) = &mut self.parent {
                parent.remove_order(order);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size == Decimal::new(0, 0)
    }
}

#[derive(Debug)]
pub struct LimitOrderBook {
    pub bids: BTreeMap<Decimal, Rc<RefCell<Limit>>>,
    pub asks: BTreeMap<Decimal, Rc<RefCell<Limit>>>,
    pub orders: HashMap<u64, Order>,
    pub lowest_ask: Option<Decimal>,
    pub highest_bid: Option<Decimal>,
}

impl LimitOrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            lowest_ask: None,
            highest_bid: None,
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.orders.insert(order.exchange_id, order.clone());

        match order.order_type {
            OrderType::Bid => {
                if let Some(limit) = self.bids.get_mut(&order.limit_price) {
                    limit.borrow_mut().add_order(order);
                } else {
                    let limit = Rc::new(RefCell::new(Limit::new(order.limit_price)));
                    limit.borrow_mut().add_order(order.clone());
                    self.bids.insert(order.limit_price, limit);
                }
            }
            OrderType::Ask => {
                if let Some(limit) = self.asks.get_mut(&order.limit_price) {
                    limit.borrow_mut().add_order(order);
                } else {
                    let limit = Rc::new(RefCell::new(Limit::new(order.limit_price)));
                    limit.borrow_mut().add_order(order.clone());
                    self.asks.insert(order.limit_price, limit);
                }
            }
        }

        self.lowest_ask = self.asks.keys().next().cloned();
        self.highest_bid = self.bids.keys().next_back().cloned();
    }

    pub fn remove_order(&mut self, order: Order) {
        let limit_price = order.limit_price;

        match order.order_type {
            OrderType::Bid => {
                if let Some(limit) = self.bids.get_mut(&limit_price) {
                    limit.borrow_mut().remove_order(order.clone());

                    if limit.borrow().is_empty() {
                        self.bids.remove(&limit_price);
                    }
                }
            }
            OrderType::Ask => {
                if let Some(limit) = self.asks.get_mut(&limit_price) {
                    limit.borrow_mut().remove_order(order.clone());

                    if limit.borrow().is_empty() {
                        self.asks.remove(&limit_price);
                    }
                }
            }
        }

        self.orders.remove(&order.exchange_id);

        self.lowest_ask = self.asks.keys().next().cloned();
        self.highest_bid = self.bids.keys().next_back().cloned();
    }

    pub fn execute_order(&mut self, order: Order) {
        let mut order = order;
        let mut limit_price = order.limit_price;

        let mut removed_limit = None;

        match order.order_type {
            OrderType::Bid => {
                while let Some(limit) = self.asks.get_mut(&limit_price) {
                    let mut limit = limit.borrow_mut();
                    if limit.size >= order.shares {
                        limit.remove_order(order.clone());
                        if limit.is_empty() {
                            removed_limit = Some(limit_price);
                        }
                        break;
                    } else {
                        order.shares -= limit.size;
                        limit.remove_order(order.clone());
                        if limit.is_empty() {
                            removed_limit = Some(limit_price);
                        }
                        limit_price = limit.limit_price;
                    }
                }
            }
            OrderType::Ask => {
                while let Some(limit) = self.bids.get_mut(&limit_price) {
                    let mut limit = limit.borrow_mut();
                    if limit.size >= order.shares {
                        limit.remove_order(order.clone());
                        if limit.is_empty() {
                            removed_limit = Some(limit_price);
                        }
                        break;
                    } else {
                        order.shares -= limit.size;
                        limit.remove_order(order.clone());
                        if limit.is_empty() {
                            removed_limit = Some(limit_price);
                        }
                        limit_price = limit.limit_price;
                    }
                }
            }
        }

        if let Some(limit_price) = removed_limit {
            match order.order_type {
                OrderType::Bid => {
                    self.asks.remove(&limit_price);
                }
                OrderType::Ask => {
                    self.bids.remove(&limit_price);
                }
            }
        }

        self.lowest_ask = self.asks.keys().next().cloned();
        self.highest_bid = self.bids.keys().next_back().cloned();
    }

    pub fn get_order(&self, exchange_id: u64) -> Option<&Order> {
        self.orders.get(&exchange_id)
    }

    pub fn get_bid_depth(&self, limit_price: Decimal) -> Decimal {
        let mut depth = Decimal::new(0, 0);
        for (price, limit) in self.bids.range(limit_price..=limit_price) {
            depth += limit.borrow().size;
        }
        depth
    }

    pub fn get_ask_depth(&self, limit_price: Decimal) -> Decimal {
        let mut depth = Decimal::new(0, 0);
        for (price, limit) in self.asks.range(limit_price..=limit_price) {
            depth += limit.borrow().size;
        }
        depth
    }

    pub fn get_bid_volume(&self, limit_price: Decimal) -> Decimal {
        let mut volume = Decimal::new(0, 0);
        for (price, limit) in self.bids.range(limit_price..=limit_price) {
            volume += limit.borrow().total_volume;
        }
        volume
    }

    pub fn get_ask_volume(&self, limit_price: Decimal) -> Decimal {
        let mut volume = Decimal::new(0, 0);
        for (price, limit) in self.asks.range(limit_price..=limit_price) {
            volume += limit.borrow().total_volume;
        }
        volume
    }

    pub fn get_bid_count(&self, limit_price: Decimal) -> usize {
        let mut count = 0;
        for (price, limit) in self.bids.range(limit_price..=limit_price) {
            count += limit.borrow().order_count;
        }
        count.try_into().unwrap()
    }

    pub fn get_ask_count(&self, limit_price: Decimal) -> usize {
        let mut count = 0;
        for (price, limit) in self.asks.range(limit_price..=limit_price) {
            count += limit.borrow().order_count;
        }
        count.try_into().unwrap()
    }

    pub fn get_bid_orders(&self, limit_price: Decimal) -> Vec<Order> {
        let mut orders = Vec::new();
        for (_, limit) in self.bids.range(limit_price..=limit_price) {
            orders.extend(limit.borrow().orders.values().cloned());
        }
        orders
    }

    pub fn get_ask_orders(&self, limit_price: Decimal) -> Vec<Order> {
        let mut orders = Vec::new();
        for (_, limit) in self.asks.range(limit_price..=limit_price) {
            orders.extend(limit.borrow().orders.values().cloned());
        }
        orders
    }

    pub fn get_spread(&self) -> Option<Decimal> {
        match (self.highest_bid, self.lowest_ask) {
            (Some(highest_bid), Some(lowest_ask)) => Some(lowest_ask - highest_bid),
            _ => None,
        }
    }

    pub fn get_mid_price(&self) -> Option<Decimal> {
        match (self.highest_bid, self.lowest_ask) {
            (Some(highest_bid), Some(lowest_ask)) => Some((lowest_ask + highest_bid) / dec!(2)),
            _ => None,
        }
    }

    pub fn get_best_bid(&self) -> Option<Decimal> {
        self.highest_bid
    }

    pub fn get_best_ask(&self) -> Option<Decimal> {
        self.lowest_ask
    }

    pub fn get_bids(&self) -> Vec<Decimal> {
        self.bids.keys().cloned().collect()
    }

    pub fn get_asks(&self) -> Vec<Decimal> {
        self.asks.keys().cloned().collect()
    }

    pub fn get_volume_at_price(&self, limit_price: Decimal) -> Option<Decimal> {
        match (self.bids.get(&limit_price), self.asks.get(&limit_price)) {
            (Some(bid), Some(ask)) => Some(bid.borrow().total_volume + ask.borrow().total_volume),
            (Some(bid), None) => Some(bid.borrow().total_volume),
            (None, Some(ask)) => Some(ask.borrow().total_volume),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_new() {
        let limit = Limit::new(dec!(100));
        assert_eq!(limit.limit_price, dec!(100));
        assert!(limit.orders.is_empty());
        assert!(limit.parent.is_none());
        assert_eq!(limit.size, dec!(0));
        assert_eq!(limit.total_volume, dec!(0));
        assert_eq!(limit.order_count, 0);
    }

    #[test]
    fn test_limit_add_remove_order() {
        let mut limit = Limit::new(dec!(100));
        let order1 = Order::new(
            "tick1".into(),
            1,
            OrderType::Bid,
            dec!(10),
            dec!(100),
            Utc::now(),
            Utc::now(),
        );
        let order2 = Order::new(
            "tick2".into(),
            2,
            OrderType::Bid,
            dec!(20),
            dec!(100),
            Utc::now(),
            Utc::now(),
        );
        let order3 = Order::new(
            "tick3".into(),
            3,
            OrderType::Ask,
            dec!(10),
            dec!(110),
            Utc::now(),
            Utc::now(),
        );

        // Add orders to the limit
        limit.add_order(order1.clone());
        assert_eq!(limit.size, dec!(10));
        assert_eq!(limit.total_volume, dec!(1000));
        assert_eq!(limit.order_count, 1);
        limit.add_order(order2.clone());
        assert_eq!(limit.size, dec!(30));
        assert_eq!(limit.total_volume, dec!(3000));
        assert_eq!(limit.order_count, 2);

        // Remove an order from the limit
        limit.remove_order(order1.clone());
        assert_eq!(limit.size, dec!(20));
        assert_eq!(limit.total_volume, dec!(2000));
        assert_eq!(limit.order_count, 1);

        // Remove the last order from the limit
        limit.remove_order(order2.clone());
        assert_eq!(limit.size, dec!(0));
        assert_eq!(limit.total_volume, dec!(0));
        assert_eq!(limit.order_count, 0);

        // Try to remove a non-existing order from the limit
        limit.remove_order(order3.clone());
        assert_eq!(limit.size, dec!(0));
        assert_eq!(limit.total_volume, dec!(0));
        assert_eq!(limit.order_count, 0);
    }

    #[test]
    fn test_limit_orderbook_new() {
        let book = LimitOrderBook::new();
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
        assert!(book.orders.is_empty());
        assert!(book.lowest_ask.is_none());
        assert!(book.highest_bid.is_none());
    }

    #[test]
    fn test_limit_orderbook_add_remove_order() {
        let mut book = LimitOrderBook::new();
        let order1 = Order::new(
            "tick1".into(),
            1,
            OrderType::Bid,
            dec!(10),
            dec!(100),
            Utc::now(),
            Utc::now(),
        );
        let order2 = Order::new(
            "tick2".into(),
            2,
            OrderType::Ask,
            dec!(20),
            dec!(110),
            Utc::now(),
            Utc::now(),
        );

        // Add a bid order to the book
        book.add_order(order1.clone());
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 0);
        assert_eq!(book.orders.len(), 1);
        assert_eq!(book.lowest_ask, None);
        assert_eq!(book.highest_bid, Some(dec!(100)));

        // Add an ask order to the book
        book.add_order(order2.clone());
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 1);
        assert_eq!(book.orders.len(), 2);
        assert_eq!(book.lowest_ask, Some(dec!(110)));
        assert_eq!(book.highest_bid, Some(dec!(100)));

        // Remove the bid order from the book
        book.remove_order(order1.clone());
        assert_eq!(book.bids.len(), 0);
        assert_eq!(book.asks.len(), 1);
        assert_eq!(book.orders.len(), 1);
        assert_eq!(book.lowest_ask, Some(dec!(110)));
        assert_eq!(book.highest_bid, None);

        // Remove the ask order from the book
        book.remove_order(order2.clone());
        assert_eq!(book.bids.len(), 0);
        assert_eq!(book.asks.len(), 0);
        assert_eq!(book.orders.len(), 0);
        assert_eq!(book.lowest_ask, None);
        assert_eq!(book.highest_bid, None);

        // Try to remove a non-existing order from the book
        book.remove_order(order1.clone());
        assert_eq!(book.bids.len(), 0);
        assert_eq!(book.asks.len(), 0);
        assert_eq!(book.orders.len(), 0);
        assert_eq!(book.lowest_ask, None);
        assert_eq!(book.highest_bid, None);
    }

    #[test]
    fn test_add_order() {
        let mut lob = LimitOrderBook::new();

        let bid = Order::new(
            "tick1".to_string(),
            1,
            OrderType::Bid,
            dec!(10),
            dec!(100),
            Utc::now(),
            Utc::now(),
        );
        lob.add_order(bid.clone());

        let ask = Order::new(
            "tick2".to_string(),
            2,
            OrderType::Ask,
            dec!(5),
            dec!(200),
            Utc::now(),
            Utc::now(),
        );
        lob.add_order(ask.clone());

        assert_eq!(lob.bids.len(), 1);
        assert_eq!(lob.asks.len(), 1);

        let bid_limit = lob.bids.values().next().unwrap().borrow();
        assert_eq!(bid_limit.orders.len(), 1);
        assert!(bid_limit.orders.contains_key(&1));
        assert_eq!(bid_limit.size, dec!(10));
        assert_eq!(bid_limit.total_volume, dec!(1000));
        assert_eq!(bid_limit.order_count, 1);

        let ask_limit = lob.asks.values().next().unwrap().borrow();
        assert_eq!(ask_limit.orders.len(), 1);
        assert!(ask_limit.orders.contains_key(&2));
        assert_eq!(ask_limit.size, dec!(5));
        assert_eq!(ask_limit.total_volume, dec!(1000));
        assert_eq!(ask_limit.order_count, 1);

        assert_eq!(lob.lowest_ask, Some(dec!(200)));
        assert_eq!(lob.highest_bid, Some(dec!(100)));
    }

    #[test]
    fn test_remove_order() {
        let mut lob = LimitOrderBook::new();

        let bid1 = Order::new(
            "tick1".to_string(),
            1,
            OrderType::Bid,
            dec!(10),
            dec!(100),
            Utc::now(),
            Utc::now(),
        );
        lob.add_order(bid1.clone());

        let bid2 = Order::new(
            "tick2".to_string(),
            2,
            OrderType::Bid,
            dec!(5),
            dec!(100),
            Utc::now(),
            Utc::now(),
        );
        lob.add_order(bid2.clone());

        let ask1 = Order::new(
            "tick3".to_string(),
            3,
            OrderType::Ask,
            dec!(5),
            dec!(200),
            Utc::now(),
            Utc::now(),
        );
        lob.add_order(ask1.clone());

        let ask2 = Order::new(
            "tick4".to_string(),
            4,
            OrderType::Ask,
            dec!(2),
            dec!(200),
            Utc::now(),
            Utc::now(),
        );
        lob.add_order(ask2.clone());
        println!("{:#?}", lob);

        lob.remove_order(bid1.clone());

        println!("{:#?}", lob);

        assert_eq!(lob.bids.len(), 1);
        assert_eq!(lob.asks.len(), 1);

        let bid_limit = lob.bids.values().next().unwrap().borrow();
        assert_eq!(bid_limit.orders.len(), 1);
        assert!(bid_limit.orders.contains_key(&2));
        assert_eq!(bid_limit.size, dec!(5));
        assert_eq!(bid_limit.total_volume, dec!(500));
        assert_eq!(bid_limit.order_count, 1);

        let ask_limit = lob.asks.values().next().unwrap().borrow();
        assert_eq!(ask_limit.orders.len(), 2);
        assert!(ask_limit.orders.contains_key(&3));
        assert_eq!(ask_limit.size, dec!(7));
        assert_eq!(ask_limit.total_volume, dec!(1400));
        assert_eq!(ask_limit.order_count, 2);

        assert_eq!(lob.lowest_ask, Some(dec!(200)));
        assert_eq!(lob.highest_bid, Some(dec!(100)));
    }

    #[test]
    fn test_execute_order() {
        let mut book = LimitOrderBook::new();

        let order1 = Order::new(
            "tick1".to_string(),
            1,
            OrderType::Bid,
            dec!(100),
            dec!(10),
            Utc::now(),
            Utc::now(),
        );

        book.add_order(order1.clone());

        let order2 = Order::new(
            "tick2".to_string(),
            2,
            OrderType::Bid,
            dec!(50),
            dec!(10),
            Utc::now(),
            Utc::now(),
        );

        book.add_order(order2.clone());

        let order3 = Order::new(
            "tick3".to_string(),
            3,
            OrderType::Ask,
            dec!(75),
            dec!(9),
            Utc::now(),
            Utc::now(),
        );

        book.add_order(order3.clone());

        let order4 = Order::new(
            "tick4".to_string(),
            4,
            OrderType::Ask,
            dec!(100),
            dec!(8),
            Utc::now(),
            Utc::now(),
        );

        book.add_order(order4.clone());

        let order5 = Order::new(
            "tick5".to_string(),
            5,
            OrderType::Bid,
            dec!(200),
            dec!(10),
            Utc::now(),
            Utc::now(),
        );

        book.execute_order(order5.clone());

        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 2);
        assert_eq!(book.orders.len(), 4);
        assert_eq!(book.lowest_ask, Some(dec!(8)));
        assert_eq!(book.highest_bid, Some(dec!(10)));
    }
}

use rust_decimal::prelude::*;
use std::collections::HashMap;

#[derive(Debug)]
pub enum OrderType {
    Bid,
    Ask,
}

#[derive(Debug)]
pub struct OrderBook {
    bids: HashMap<Decimal, LimitOrder>,
    asks: HashMap<Decimal, LimitOrder>,
}

impl OrderBook {
    pub fn new() -> OrderBook {
        OrderBook {
            bids: HashMap::new(),
            asks: HashMap::new(),
        }
    }

    pub fn fill_market_order(&mut self, market_order: &mut Order) {
        let limits = match market_order.order_type {
            OrderType::Bid => self.ask_limits(),
            OrderType::Ask => self.bid_limits(),
        };

        for limit_order in limits {
            limit_order.fill_order(market_order);

            if market_order.is_filled() {
                break;
            }
        }
    }

    pub fn ask_limits(&mut self) -> Vec<&mut LimitOrder> {
        let mut limits = self.asks.values_mut().collect::<Vec<&mut LimitOrder>>();
        limits.sort_by(|a, b| a.price.cmp(&b.price));
        limits
    }

    pub fn bid_limits(&mut self) -> Vec<&mut LimitOrder> {
        let mut limits = self.bids.values_mut().collect::<Vec<&mut LimitOrder>>();
        limits.sort_by(|a, b| b.price.cmp(&a.price));
        limits
    }

    pub fn add_limit_order(&mut self, order: Order, price: Decimal) {
        match order.order_type {
            OrderType::Bid => match self.bids.get_mut(&price) {
                Some(limit_order) => limit_order.add_order(order),
                None => {
                    let mut limit_order = LimitOrder::new(price.clone());
                    limit_order.add_order(order);
                    self.bids.insert(price, limit_order);
                }
            },
            OrderType::Ask => match self.asks.get_mut(&price) {
                Some(limit_order) => limit_order.add_order(order),
                None => {
                    let mut limit_order = LimitOrder::new(price.clone());
                    limit_order.add_order(order);
                    self.asks.insert(price, limit_order);
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct LimitOrder {
    price: Decimal,
    orders: Vec<Order>,
}

impl LimitOrder {
    pub fn new(price: Decimal) -> LimitOrder {
        LimitOrder {
            price,
            orders: Vec::new(),
        }
    }

    fn total_volume(&self) -> f64 {
        self.orders
            .iter()
            .map(|order| order.size)
            .reduce(|a, b| a + b)
            .unwrap()
    }

    fn fill_order(&mut self, market_order: &mut Order) {
        for limit_order in self.orders.iter_mut() {
            match market_order.size >= limit_order.size {
                true => {
                    market_order.size -= limit_order.size;
                    limit_order.size = 0.0
                }
                false => {
                    limit_order.size -= market_order.size;
                    market_order.size = 0.0
                }
            }

            if market_order.is_filled() {
                break;
            }
        }
    }

    fn add_order(&mut self, order: Order) {
        self.orders.push(order);
    }
}

#[derive(Debug)]
pub struct Order {
    size: f64,
    order_type: OrderType,
}

impl Order {
    pub fn new(size: f64, order_type: OrderType) -> Order {
        Order { size, order_type }
    }

    pub fn is_filled(&self) -> bool {
        self.size == 0.0
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn orderbook_fill_ask_order() {
        let mut orderbook = OrderBook::new();
        orderbook.add_limit_order(Order::new(10.0, OrderType::Ask), dec!(500));
        orderbook.add_limit_order(Order::new(10.0, OrderType::Ask), dec!(200));
        orderbook.add_limit_order(Order::new(10.0, OrderType::Ask), dec!(100));
        orderbook.add_limit_order(Order::new(10.0, OrderType::Ask), dec!(150));
        orderbook.add_limit_order(Order::new(10.0, OrderType::Ask), dec!(50));

        let mut market_order = Order::new(10.0, OrderType::Bid);
        orderbook.fill_market_order(&mut market_order);

        let ask_limits = orderbook.ask_limits();
        let matched_limit = ask_limits.get(0).unwrap();

        assert_eq!(matched_limit.price, dec!(50));
        assert_eq!(market_order.is_filled(), true);

        let matched_order = matched_limit.orders.get(0).unwrap();
        assert_eq!(matched_order.is_filled(), true);
    }

    #[test]
    fn limit_total_volume() {
        let price = dec!(100000.0);
        let mut limit = LimitOrder::new(price);
        let buy_limit_order_a = Order::new(100.0, OrderType::Bid);
        let buy_limit_order_b = Order::new(100.0, OrderType::Bid);

        limit.add_order(buy_limit_order_a);
        limit.add_order(buy_limit_order_b);

        assert_eq!(limit.total_volume(), 200.0)
    }

    #[test]
    fn limit_order_multiple_fill() {
        let price = dec!(100000.0);
        let mut limit = LimitOrder::new(price);
        let buy_limit_order_a = Order::new(100.0, OrderType::Bid);
        let buy_limit_order_b = Order::new(100.0, OrderType::Bid);
        limit.add_order(buy_limit_order_a);
        limit.add_order(buy_limit_order_b);

        let mut market_sell_order = Order::new(199.0, OrderType::Ask);
        limit.fill_order(&mut market_sell_order);

        assert_eq!(market_sell_order.is_filled(), true);
        assert_eq!(limit.orders.get(0).unwrap().is_filled(), true);
        assert_eq!(limit.orders.get(1).unwrap().is_filled(), false);
        assert_eq!(limit.orders.get(1).unwrap().size, 1.0);
    }

    #[test]
    fn limit_order_single_fill() {
        let price = dec!(100000.0);
        let mut limit = LimitOrder::new(price);
        let buy_limit_order = Order::new(100.0, OrderType::Bid);
        limit.add_order(buy_limit_order);

        let mut market_sell_order = Order::new(99.0, OrderType::Ask);
        limit.fill_order(&mut market_sell_order);

        assert_eq!(market_sell_order.is_filled(), true);
        assert_eq!(limit.orders.get(0).unwrap().size, 1.0);
    }
}

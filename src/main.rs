mod limit_order_book;
mod matching_engine;
use matching_engine::orderbook::{Order, OrderBook, OrderType};
use rust_decimal_macros::dec;

fn main() {
    let buy_order_1 = Order::new(1.0, OrderType::Bid);
    let buy_order_2 = Order::new(2.0, OrderType::Bid);
    let sell_order_1 = Order::new(1.0, OrderType::Ask);
    let sell_order_2 = Order::new(2.0, OrderType::Ask);

    let mut order_book = OrderBook::new();
    order_book.add_limit_order(buy_order_1, dec!(1.0));
    order_book.add_limit_order(buy_order_2, dec!(1.0));
    order_book.add_limit_order(sell_order_1, dec!(2.0));
    order_book.add_limit_order(sell_order_2, dec!(2.0));

    println!("{:?}", order_book);
}

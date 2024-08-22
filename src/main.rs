use coin_trade_sdk::binance::{Binance, BinanceTrait};
use coin_trade_sdk::bithumb::{Bithumb, BithumbTrait};
use coin_trade_sdk::okx::{Okx, OkxTrait};
use coin_trade_sdk::upbit::{Upbit, UpbitTrait};
use coin_trade_sdk::Exchange;
use serde_json::json;
use tokio;

#[tokio::main]
async fn main() {
    let upbit_exchange: Box<dyn Exchange> = Box::new(Upbit::new("asdf".to_string(), "asdf".to_string()).unwrap());
    let binance_exchange: Box<dyn Exchange> = Box::new(Binance::new("asdf".to_string(), "asdf".to_string()).unwrap());
    let okx_exchange: Box<dyn Exchange> = Box::new(Okx::new("asdf".to_string(), "asdf".to_string(), "asdf".to_string()).unwrap());
    let bithumb_exchange: Box<dyn Exchange> = Box::new(Bithumb::new("asdf".to_string(), "asdf".to_string()).unwrap());

    let exchanges: Vec<Box<dyn Exchange>> = vec![upbit_exchange, binance_exchange, okx_exchange, bithumb_exchange];

    for exchange in exchanges {
        let res = exchange.place_order(json!({
            "symbol": "BTCUSDT",
            "side": "bid",
            "order_type": "limit",
            "price": "10.0",
            "amount": "1.0"
        })).await.unwrap();
        println!("{}_place_order:{}", exchange.get_name(), res.to_string());

        let res = exchange.cancel_order(json!({
            "symbol": "BTCUSDT",
            "order_id": "4"
        })).await.unwrap();
        println!("{}_cancel_order:{}\n", exchange.get_name(), res.to_string());
    }
}

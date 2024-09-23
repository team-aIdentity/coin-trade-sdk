use coin_trade_sdk::binance::{ Binance, BinanceTrait };
use coin_trade_sdk::bithumb::{ Bithumb, BithumbTrait };
use coin_trade_sdk::okx::{ Okx, OkxTrait };
use coin_trade_sdk::upbit::{ Upbit, UpbitTrait };
use coin_trade_sdk::{ Exchange, OrderBook, OrderBookUnit };
use dotenv::dotenv;
use serde_json::json;
use tokio;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let upbit_api_key = std::env::var("UPBIT_API_KEY").expect("UPBIT_API_KEY must be set.");
    let upbit_secret = std::env::var("UPBIT_SECRET").expect("UPBIT_SECRET must be set.");
    let bithumb_api_key = std::env::var("BITHUMB_API_KEY").expect("BITHUMB_API_KEY must be set.");
    let bithumb_secret = std::env::var("BITHUMB_SECRET").expect("BITHUMB_SECRET must be set.");
    let okx_api_key = std::env::var("OKX_API_KEY").expect("OKX_API_KEY must be set.");
    let okx_secret = std::env::var("OKX_SECRET").expect("OKX_SECRET must be set.");
    let okx_passphrase = std::env::var("OKX_PASSPHRASE").expect("OKX_PASSPHRASE must be set.");
    let binance_api_key = std::env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY must be set.");
    let binance_secret = std::env::var("BINANCE_SECRET").expect("BINANCE_SECRET must be set.");

    let upbit_exchange: Box<dyn Exchange> = Box::new(
        Upbit::new(upbit_api_key, upbit_secret).unwrap()
    );
    let binance_exchange: Box<dyn Exchange> = Box::new(
        Binance::new(bithumb_api_key, bithumb_secret).unwrap()
    );
    let okx_exchange: Box<dyn Exchange> = Box::new(
        Okx::new(okx_api_key, okx_secret, okx_passphrase).unwrap()
    );
    let bithumb_exchange: Box<dyn Exchange> = Box::new(
        Bithumb::new(binance_api_key, binance_secret).unwrap()
    );

    let exchanges: Vec<Box<dyn Exchange>> = vec![
        upbit_exchange,
        binance_exchange,
        bithumb_exchange,
        okx_exchange
    ];

    for exchange in exchanges {
        let res = exchange
            .place_order(
                json!({
            "symbol": "BTC/KRW",
            "side": "bid",
            "order_type": "limit",
            "price": "100000",
            "amount": "1.0"
        })
            ).await
            .unwrap();
        println!("{}_place_order:{}", exchange.get_name(), res.to_string());

        let res = exchange
            .cancel_order(
                json!({
            "symbol": "BTC/USDT",
            "order_id": "4"
        })
            ).await
            .unwrap();
        println!("{}_cancel_order:{}", exchange.get_name(), res.to_string());

        let res: OrderBook;
        if exchange.get_name() == "Bithumb" {
            res = exchange
                .get_order_book(
                    json!({
                    "symbol": "BTC/KRW",
                })
                ).await
                .unwrap();
        } else {
            res = exchange
                .get_order_book(
                    json!({
                    "symbol": "ETH/USDT",
                })
                ).await
                .unwrap();
        }

        println!("{}_get_order_book:{:?}\n", exchange.get_name(), res);
    }
}

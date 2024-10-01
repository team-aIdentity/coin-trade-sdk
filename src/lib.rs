use std::collections::BTreeMap;
use std::time::{ SystemTime, UNIX_EPOCH };

use async_trait::async_trait;
use http::{ Request, Version };
use reqwest::{ Client, Response };
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use url::Url;

pub mod binance;
pub mod bithumb;
pub mod okx;
pub mod upbit;

#[async_trait]
pub trait Exchange {
    async fn place_order(&self, req: Value) -> Result<Value, String>;
    async fn cancel_order(&self, req: Value) -> Result<Value, String>;
    async fn get_order_book(&self, req: Value) -> Result<OrderBook, String>;
    fn get_name(&self) -> String;
    async fn get_current_price(&self, req: Value) -> Result<Price, String>;
    async fn get_coin_list(&self) -> Result<CoinList, String>;
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Order {
    pub exchange: String,
    pub ord_id: String,
    pub side: String,
    pub ord_type: String,
    pub price: String,
    pub state: String,
    pub market: String,
    pub volume: String,
    pub create_at: String,
    pub amount: String,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Price {
    pub exchange: String,
    pub symbol: String,
    pub price: String,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct OrderBookUnit {
    pub ask_price: String,
    pub bid_price: String,
    pub ask_size: String,
    pub bid_size: String,
}

#[derive(Debug)]
pub struct OrderBook {
    pub market: String,
    pub exchange: String,
    pub orderbook_unit: Vec<OrderBookUnit>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct CoinList {
    pub market: String,
    pub coin_list: Vec<String>,
}

async fn send(req: Request<BTreeMap<&str, &str>>) -> Result<http::Response<Vec<u8>>, String> {
    let client = Client::new();
    let uri = req.uri().to_string();
    let url = Url::parse(&uri).unwrap();

    let headers = req.headers().clone();
    let content_type = headers
        .get("Content-Type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json");

    let mut request_builder = client.request(req.method().clone(), url);

    match content_type {
        "application/x-www-form-urlencoded" => {
            let mut form_data = BTreeMap::new();
            for (key, value) in req.body() {
                form_data.insert(key.to_string(), value.to_string());
            }
            request_builder = request_builder.form(&form_data);
        }
        "application/json" => {
            let json_body = serde_json::to_value(req.body()).map_err(|e| e.to_string())?;
            request_builder = request_builder.json(&json_body);
        }
        _ => {
            return Err("Unsupported Content-Type".into());
        }
    }

    let request = request_builder
        .headers(headers)
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.execute(request).await.unwrap();

    let convert_reqwest_to_http = convert_reqwest_to_http(response).await;
    Ok(convert_reqwest_to_http)
}

async fn convert_reqwest_to_http(response: Response) -> http::Response<Vec<u8>> {
    let status = response.status();
    let headers = response.headers().clone();
    let version = match response.version() {
        reqwest::Version::HTTP_09 => Version::HTTP_09,
        reqwest::Version::HTTP_10 => Version::HTTP_10,
        reqwest::Version::HTTP_11 => Version::HTTP_11,
        reqwest::Version::HTTP_2 => Version::HTTP_2,
        _ => Version::default(),
    };

    let body = response.bytes().await.expect("Failed to get response body").to_vec();

    let mut builder = http::Response::builder().status(status).version(version);

    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }

    builder.body(body).expect("Failed to build HTTP response")
}

fn get_current_timestamp_in_millis() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    since_the_epoch.as_millis() as u64
}

pub fn get_query_string(param: BTreeMap<&str, &str>) -> String {
    param
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<String>>()
        .join("&")
}

#[cfg(test)]
mod test;

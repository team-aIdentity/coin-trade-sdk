use std::collections::BTreeMap;
use async_trait::async_trait;
use http::{ header::{ ACCEPT, CONTENT_TYPE }, Request };
use serde_json::{ from_slice, Value };
use sha2::{ Digest, Sha256 };
use hmac::{ Hmac, Mac };
use base64::{ Engine as _, engine::general_purpose };

use crate::{
    get_current_timestamp_in_millis,
    get_query_string,
    send,
    Exchange,
    OrderBook,
    OrderBookUnit,
};

pub struct Okx {
    api_url: String,
    api_key: String,
    secret: String,
    passphrase: String,
    endpoint: BTreeMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait OkxTrait {
    fn new(api_key: String, secret: String, passphrase: String) -> Result<Self, String>
        where Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &BTreeMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
    fn send_req_with_sign(
        &self,
        param: BTreeMap<&str, &str>,
        endpoint_key: &str
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
}

impl Okx {
    fn validate_api_credentials(
        api_key: &str,
        secret: &str,
        passphrase: &str
    ) -> Result<(), String> {
        if api_key.is_empty() || secret.is_empty() || passphrase.is_empty() {
            return Err("API key, Secret, and Passphrase cannot be empty".to_string());
        }
        Ok(())
    }

    fn create_hmac_key(&self) -> Result<Hmac<Sha256>, String> {
        Hmac::new_from_slice(self.secret.as_bytes()).map_err(|e| e.to_string())
    }

    fn build_request<'a>(
        &'a self,
        method: &str,
        uri: &str,
        headers: Vec<(http::HeaderName, &str)>,
        body: BTreeMap<&'a str, &'a str>
    ) -> Result<Request<BTreeMap<&str, &str>>, String> {
        let mut builder = Request::builder().method(method).uri(uri);
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        builder.body(body).map_err(|e| e.to_string())
    }

    fn get_signature(
        &self,
        params: &BTreeMap<&str, &str>,
        timestamp: &str,
        method: &str,
        endpoint: &str
    ) -> Result<String, String> {
        let query_string = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join("&");

        let mut mac = self.create_hmac_key()?;
        mac.update((timestamp.to_string() + method + endpoint + "?" + &query_string).as_bytes());

        let hmac_bytes = mac.finalize().into_bytes();
        let b64 = general_purpose::STANDARD.encode(hmac_bytes);

        Ok(b64)
    }
}

impl OkxTrait for Okx {
    fn new(api_key: String, secret: String, passphrase: String) -> Result<Self, String> {
        Okx::validate_api_credentials(&api_key, &secret, &passphrase)?;

        let endpoint = BTreeMap::from([
            ("make_order".to_string(), ["POST".to_string(), "api/v5/trade/order".to_string()]),
            (
                "cancel_order".to_string(),
                ["POST".to_string(), "api/v5/trade/cancel-order".to_string()],
            ),
            ("order_book".to_string(), ["GET".to_string(), "api/v5/market/books-full".to_string()]),
            ("current_price".to_string(), ["GET".to_string(), "api/v5/market/price".to_string()]),
        ]);

        Ok(Self {
            api_url: "https://www.okx.com/".to_string(),
            api_key,
            secret,
            passphrase,
            endpoint,
        })
    }

    fn get_api_url(&self) -> &str {
        &self.api_url
    }

    fn get_end_point(&self) -> &BTreeMap<String, [String; 2]> {
        &self.endpoint
    }

    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]> {
        self.endpoint.get(key)
    }

    async fn send_req_with_sign(
        &self,
        param: BTreeMap<&str, &str>,
        endpoint_key: &str
    ) -> Result<Value, String> {
        let timestamp = get_current_timestamp_in_millis().to_string();
        let authorization = self.get_signature(&param, &timestamp, "POST", endpoint_key)?;

        let base = self
            .get_end_point_with_key(endpoint_key)
            .ok_or("Endpoint not found".to_string())?;

        let uri = format!("{}{}", self.api_url, base[1]);
        let request = self.build_request(
            base[0].as_str(),
            &uri,
            vec![
                ("OK-ACCESS-KEY".parse().unwrap(), &self.api_key),
                ("OK-ACCESS-SIGN".parse().unwrap(), &authorization),
                ("OK-ACCESS-TIMESTAMP".parse().unwrap(), &timestamp),
                ("OK-ACCESS-PASSPHRASE".parse().unwrap(), &self.passphrase),
                (CONTENT_TYPE, "application/json")
            ],
            param
        )?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        from_slice(&body).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Exchange for Okx {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap_or_default()); // 심볼 파싱
        let params = BTreeMap::from([
            ("instId", symbol.as_str()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("ordType", req["order_type"].as_str().unwrap_or_default()),
            ("px", req["price"].as_str().unwrap_or_default()),
            ("sz", req["amount"].as_str().unwrap_or_default()),
            ("tdMode", "cash"),
        ]);

        self.send_req_with_sign(params, "make_order").await
    }

    async fn cancel_order(&self, req: Value) -> Result<Value, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap_or_default()); // 심볼 파싱
        let params = BTreeMap::from([
            ("instId", symbol.as_str()),
            ("ordId", req["order_id"].as_str().unwrap_or_default()),
        ]);

        self.send_req_with_sign(params, "cancel_order").await
    }

    async fn get_order_book(&self, req: Value) -> Result<OrderBook, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap_or_default()); // 심볼 파싱
        let params = BTreeMap::from([
            ("instId", symbol.as_str()),
            ("sz", "30"),
        ]);

        let base = self
            .get_end_point_with_key("order_book")
            .ok_or("Endpoint not found".to_string())?;

        let query_string = get_query_string(params);
        let uri = format!("{}{}?{}", self.api_url, base[1], query_string);

        let request = self.build_request(
            base[0].as_str(),
            &uri,
            vec![(ACCEPT, "application/json")],
            BTreeMap::new()
        )?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();

        let res: Value = from_slice(&body).map_err(|e| format!("Failed to parse response: {}", e))?;

        let orderbook = parse_orderbook(res, req["symbol"].as_str().unwrap().to_string())?;
        Ok(orderbook)
    }

    fn get_name(&self) -> String {
        "Okx".to_string()
    }

    async fn get_current_price(&self, req: Value) -> Result<Value, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap_or_default()); // 심볼 파싱
        let params = BTreeMap::from([
            ("instId", symbol.as_str()),
            ("sz", "30"),
        ]);

        let base = self
            .get_end_point_with_key("current_price")
            .ok_or("Endpoint not found".to_string())?;

        let query_string = get_query_string(params);
        let uri = format!("{}{}?{}", self.api_url, base[1], query_string);

        let request = self.build_request(
            base[0].as_str(),
            &uri,
            vec![(ACCEPT, "application/json")],
            BTreeMap::new()
        )?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();

        let res: Value = from_slice(&body).map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(res)
    }
}

fn parse_symbol(symbol: &str) -> String {
    let v: Vec<&str> = symbol.split("/").collect();
    format!("{}-{}", v[0], v[1])
}

fn encode_symbol(symbol: &str) -> String {
    println!("{}", symbol);
    let v: Vec<&str> = symbol.split("-").collect();
    format!("{}/{}", v[0], v[1])
}

fn parse_orderbook(orderbook_res: Value, symbol: String) -> Result<OrderBook, String> {
    let orderbook_unit: Vec<OrderBookUnit> = orderbook_res["data"][0]["bids"]
        .as_array()
        .ok_or("Failed to parse orderbook bids")?
        .iter()
        .map(|unit| {
            let ask_price = unit[0].as_str().unwrap_or_default().to_string();
            let ask_size = unit[1].as_str().unwrap_or_default().to_string();
            OrderBookUnit {
                ask_price: ask_price.clone(),
                bid_price: ask_price.clone(), // Assuming a symmetrical book
                ask_size: ask_size.clone(),
                bid_size: ask_size.clone(),
            }
        })
        .collect::<Vec<OrderBookUnit>>();

    Ok(OrderBook {
        market: symbol, // encode_symbol을 사용하여 심볼을 반환
        exchange: "Okx".to_string(),
        orderbook_unit,
    })
}

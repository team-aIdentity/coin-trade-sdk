use std::collections::BTreeMap;
use async_trait::async_trait;
use serde_json::{ from_slice, Value };
use http::{ header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE }, HeaderName, Request };
use sha2::{ Digest, Sha512 };
use uuid::Uuid;
use hmac::{ Hmac, Mac };
use jwt::SignWithKey;

use crate::{ get_query_string, send, CoinList, Exchange, OrderBook, OrderBookUnit, Price };

pub struct Bithumb {
    api_url: String,
    api_key: String,
    secret: String,
    endpoint: BTreeMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait BithumbTrait {
    fn new(api_key: String, secret: String) -> Result<Self, String> where Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &BTreeMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
    fn send_req_with_sign(
        &self,
        param: BTreeMap<&str, &str>,
        endpoint_key: &str
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
}

impl Bithumb {
    fn validate_api_credentials(api_key: &str, secret: &str) -> Result<(), String> {
        if api_key.is_empty() || secret.is_empty() {
            return Err("API key and Secret cannot be empty".to_string());
        }
        Ok(())
    }

    fn create_hmac_key(&self) -> Result<Hmac<Sha512>, String> {
        Hmac::new_from_slice(self.secret.as_bytes()).map_err(|e| e.to_string())
    }

    fn build_request<'a>(
        &'a self,
        method: &str,
        uri: &str,
        headers: Vec<(HeaderName, &str)>,
        body: BTreeMap<&'a str, &'a str>
    ) -> Result<Request<BTreeMap<&'a str, &'a str>>, String> {
        let mut builder = Request::builder().method(method).uri(uri);
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        builder.body(body).map_err(|e| e.to_string())
    }

    fn get_authorization_header(&self, param: BTreeMap<&str, &str>) -> Result<String, String> {
        let query = get_query_string(param.clone());

        let mut hasher = Sha512::new();
        hasher.update(query.as_bytes());
        let query_hash = hex::encode(hasher.finalize());

        let nonce = Uuid::new_v4().to_string();
        let payload = BTreeMap::from([
            ("access_key", self.api_key.clone()),
            ("nonce", nonce),
            ("query_hash", query_hash),
            ("query_hash_alg", "SHA512".to_string()),
        ]);

        let key = self.create_hmac_key()?;
        let jwt_token = payload.sign_with_key(&key).map_err(|e| e.to_string())?;

        Ok(format!("Bearer {}", jwt_token))
    }
}

impl BithumbTrait for Bithumb {
    fn new(api_key: String, secret: String) -> Result<Self, String> {
        Bithumb::validate_api_credentials(&api_key, &secret)?;

        let endpoint = BTreeMap::from([
            ("make_order".to_string(), ["POST".to_string(), "v1/orders".to_string()]),
            ("cancel_order".to_string(), ["DELETE".to_string(), "v1/order".to_string()]),
            ("order_book".to_string(), ["GET".to_string(), "v1/orderbook".to_string()]),
            ("current_price".to_string(), ["GET".to_string(), "v1/ticker".to_string()]),
            ("coin_list".to_string(), ["GET".to_string(), "v1/market/all".to_string()]),
        ]);

        Ok(Self {
            api_url: "https://api.bithumb.com/".to_string(),
            api_key,
            secret,
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
        let authorization = self.get_authorization_header(param.clone())?;

        let base = self
            .get_end_point_with_key(endpoint_key)
            .ok_or("Endpoint not found".to_string())?;

        let uri = format!("{}{}", self.api_url, base[1]);
        let request = self.build_request(
            base[0].as_str(),
            &uri,
            vec![(AUTHORIZATION, &authorization), (CONTENT_TYPE, "application/json")],
            param
        )?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        from_slice(&body).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Exchange for Bithumb {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([
            ("market", symbol.as_str()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("ord_type", req["order_type"].as_str().unwrap_or_default()),
            ("price", req["price"].as_str().unwrap_or_default()),
            ("volume", req["amount"].as_str().unwrap_or_default()),
        ]);

        self.send_req_with_sign(params, "make_order").await
    }

    async fn cancel_order(&self, req: Value) -> Result<Value, String> {
        let params = BTreeMap::from([("uuid", req["order_id"].as_str().unwrap_or_default())]);

        self.send_req_with_sign(params, "cancel_order").await
    }

    async fn get_order_book(&self, req: Value) -> Result<OrderBook, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([("markets", symbol.as_str())]);

        let query_string = get_query_string(params);
        let base = self
            .get_end_point_with_key("order_book")
            .ok_or("Endpoint not found".to_string())?;

        let uri = format!("{}{}?{}", self.api_url, base[1], query_string);
        let request = self.build_request(
            base[0].as_str(),
            &uri,
            vec![(ACCEPT, "application/json")],
            BTreeMap::new()
        )?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let res: Value = from_slice(&body).unwrap();
        Ok(parse_orderbook(res)?)
    }

    fn get_name(&self) -> String {
        "Bithumb".to_string()
    }

    async fn get_current_price(&self, req: Value) -> Result<Price, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([("markets", symbol.as_str())]);

        let query_string = get_query_string(params);
        let base = self
            .get_end_point_with_key("current_price")
            .ok_or("Endpoint not found".to_string())?;

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

        // Parsing response to create Price struct
        let symbol_name = req["symbol"].as_str().unwrap().to_string();
        let current_price = res[0]["trade_price"].as_f64().unwrap_or(0.0).to_string();

        let price = Price {
            exchange: "Bithumb".to_string(),
            symbol: symbol_name,
            price: current_price,
        };

        Ok(price)
    }

    async fn get_coin_list(&self) -> Result<CoinList, String> {
        let params = BTreeMap::from([("isDetails", "false")]);

        let query_string = get_query_string(params);
        let base = self
            .get_end_point_with_key("coin_list")
            .ok_or("Endpoint not found".to_string())?;

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

        // Parsing response to create CoinList struct
        let market = "Bithumb".to_string();
        let coin_list = res
            .as_array()
            .ok_or("Response is not an array".to_string())?
            .iter()
            .filter_map(|coin| coin["market"].as_str().map(|s| encode_symbol(s)))
            .collect::<Vec<String>>();

        let coin_list_struct = CoinList {
            market,
            coin_list,
        };

        Ok(coin_list_struct)
    }
}

fn parse_symbol(symbol: &str) -> String {
    let v: Vec<&str> = symbol.split("/").collect();
    format!("{}-{}", v[1], v[0])
}

fn encode_symbol(symbol: &str) -> String {
    let v: Vec<&str> = symbol.split("-").collect();
    format!("{}/{}", v[1], v[0])
}

fn parse_orderbook(orderbook_res: Value) -> Result<OrderBook, String> {
    let orderbook_units = orderbook_res[0]["orderbook_units"]
        .as_array()
        .ok_or("orderbook_units field is not an array")?
        .iter()
        .map(|unit| {
            let ask_price = unit["ask_price"].as_f64().unwrap_or(0.0).to_string();
            let bid_price = unit["bid_price"].as_f64().unwrap_or(0.0).to_string();
            let ask_size = unit["ask_size"].as_f64().unwrap_or(0.0).to_string();
            let bid_size = unit["bid_size"].as_f64().unwrap_or(0.0).to_string();
            OrderBookUnit {
                ask_price,
                bid_price,
                ask_size,
                bid_size,
            }
        })
        .collect::<Vec<OrderBookUnit>>();

    let symbol = encode_symbol(orderbook_res[0]["market"].as_str().unwrap_or_default());
    Ok(OrderBook {
        market: symbol,
        exchange: "Bithumb".to_string(),
        orderbook_unit: orderbook_units,
    })
}

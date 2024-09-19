use std::collections::BTreeMap;
use async_trait::async_trait;
use http::header::CONTENT_TYPE;
use serde_json::{from_slice, Value};
use http::Request;
use sha2::Sha256;
use hmac::{Hmac, Mac};

use crate::{get_current_timestamp_in_millis, send, Exchange};

pub struct Binance {
    api_url: String,
    api_key: String,
    secret: String,
    endpoint: BTreeMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait BinanceTrait {
    fn get_signature(&self, params: &BTreeMap<&str, &str>) -> Result<String, String>;
    fn new(api_key: String, secret: String) -> Result<Self, String>
    where
        Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &BTreeMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
}

impl Binance {
    fn validate_api_credentials(api_key: &str, secret: &str) -> Result<(), String> {
        if api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if secret.is_empty() {
            return Err("Secret cannot be empty".to_string());
        }
        Ok(())
    }

    fn create_hmac_key(&self) -> Result<Hmac<Sha256>, String> {
        Hmac::new_from_slice(self.secret.as_bytes()).map_err(|e| e.to_string())
    }
}

impl BinanceTrait for Binance {
    fn new(api_key: String, secret: String) -> Result<Self, String> {
        Binance::validate_api_credentials(&api_key, &secret)?;

        let endpoint = BTreeMap::from([
            ("make_order".to_string(), ["POST".to_string(), "api/v3/order".to_string()]),
            ("cancel_order".to_string(), ["DELETE".to_string(), "api/v3/order".to_string()]),
            ("order_book".to_string(), ["GET".to_string(), "api/v3/depth".to_string()])
        ]);

        Ok(Self {
            api_url: "https://api1.binance.com/".to_string(),
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

    fn get_signature(&self, params: &BTreeMap<&str, &str>) -> Result<String, String> {
        let query_string = params.iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join("&");

        let mut mac = self.create_hmac_key()?;
        mac.update(query_string.as_bytes());

        let result = mac.finalize();
        let hmac_bytes = result.into_bytes();

        Ok(hex::encode(hmac_bytes))
    }
}

#[async_trait]
impl Exchange for Binance {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let get_current_timestamp_in_millis = get_current_timestamp_in_millis().to_string();
        let timestamp = get_current_timestamp_in_millis.as_str();
        let mut params = BTreeMap::from([
            ("symbol", req["symbol"].as_str().unwrap_or_default()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("type", req["order_type"].as_str().unwrap_or_default()),
            ("price", req["price"].as_str().unwrap_or_default()),
            ("quantity", req["amount"].as_str().unwrap_or_default()),
            ("timestamp", timestamp),
            ("newOrderRespType", "RESULT")
        ]);

        let signature = self.get_signature(&params)?;
        params.insert("signature", &signature);
        
        let base = self
            .get_end_point_with_key("make_order")
            .ok_or("Endpoint not found".to_string())?;
        
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header("X-MBX-APIKEY", self.api_key.clone())
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(params)
            .map_err(|e| e.to_string())?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let json_value: Value = from_slice(&body).map_err(|e| e.to_string())?;

        Ok(json_value)
    }

    async fn cancel_order(&self, req: Value) -> Result<Value, String> {
        let get_current_timestamp_in_millis = get_current_timestamp_in_millis().to_string();
        let timestamp = get_current_timestamp_in_millis.as_str();
        let mut params = BTreeMap::from([
            ("symbol", req["symbol"].as_str().unwrap_or_default()),
            ("orderId", req["order_id"].as_str().unwrap_or_default()),
            ("timestamp", timestamp),
        ]);

        let signature = self.get_signature(&params)?;
        params.insert("signature", &signature);
        
        let base = self
            .get_end_point_with_key("cancel_order")
            .ok_or("Endpoint not found".to_string())?;
        
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header("X-MBX-APIKEY", self.api_key.clone())
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(params)
            .map_err(|e| e.to_string())?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let json_value: Value = from_slice(&body).map_err(|e| e.to_string())?;

        Ok(json_value)
    }

    async fn get_order_book(&self, req: Value) -> Result<Value, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([
            ("symbol", symbol.as_str())
        ]);

        let query_string = params.iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join("&");

        let base = self
            .get_end_point_with_key("order_book")
            .ok_or("Endpoint not found".to_string())?;
        println!("{}{}?{}", self.api_url, base[1], query_string);
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}?{}", self.api_url, base[1], query_string))
            .body(BTreeMap::new())
            .map_err(|e| e.to_string())?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let json_value: Value = from_slice(&body).map_err(|e| e.to_string())?;

        Ok(json_value)
    }
    
    fn get_name(&self) -> String {
        "Binance".to_string()
    }
}

fn parse_symbol(symbol: &str) -> String {
    let v: Vec<&str> = symbol.split("/").collect();
    format!("{}{}", v[0], v[1])
}



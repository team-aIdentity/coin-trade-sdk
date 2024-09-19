use std::collections::BTreeMap;
use async_trait::async_trait;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{from_slice, Value};
use http::Request;
use sha2::{Sha512, Digest};
use uuid::Uuid;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use urlencoding::encode;

use crate::{get_current_timestamp_in_millis, send, Exchange};

pub struct Bithumb {
    api_url: String,
    api_key: String,
    secret: String,
    endpoint: BTreeMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait BithumbTrait {
    fn new(api_key: String, secret: String) -> Result<Self, String>
    where
        Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &BTreeMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
    fn get_json(&self, query_hash: String) -> Result<String, String>;
    fn get_query_hash(&self, params: &BTreeMap<&str, &str>) -> Result<String, String>;
}

impl Bithumb {
    fn validate_api_credentials(api_key: &str, secret: &str) -> Result<(), String> {
        if api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if secret.is_empty() {
            return Err("Secret cannot be empty".to_string());
        }
        Ok(())
    }

    fn create_hmac_key(&self) -> Result<Hmac<Sha512>, String> {
        Hmac::new_from_slice(self.secret.as_bytes()).map_err(|e| e.to_string())
    }
}

impl BithumbTrait for Bithumb {
    fn new(api_key: String, secret: String) -> Result<Self, String> {
        Bithumb::validate_api_credentials(&api_key, &secret)?;

        let endpoint = BTreeMap::from([
            ("make_order".to_string(), ["POST".to_string(), "v1/orders".to_string()]),
            ("cancel_order".to_string(), ["DELETE".to_string(), "v1/order".to_string()]),
            ("order_book".to_string(), ["GET".to_string(), "v1/orderbook".to_string()])
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

    fn get_json(&self, query_hash: String) -> Result<String, String> {
        let nonce = Uuid::new_v4().to_string();
        let payload = BTreeMap::from([
            ("access_key".to_string(), self.api_key.clone()),
            ("nonce".to_string(), nonce),
            ("timestamp".to_string(), get_current_timestamp_in_millis().to_string()),
            ("query_hash".to_string(), query_hash),
            ("query_hash_alg".to_string(), "SHA512".to_string()),
        ]);

        let key = self.create_hmac_key()?;
        payload
            .sign_with_key(&key)
            .map_err(|e| e.to_string())
    }

    fn get_query_hash(&self, params: &BTreeMap<&str, &str>) -> Result<String, String> {
        let serialized_params = serde_json::to_string(params).map_err(|e| e.to_string())?;
        let query_string = encode(&serialized_params);

        let mut hasher = Sha512::new();
        hasher.update(query_string.as_bytes());
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }
}

#[async_trait]
impl Exchange for Bithumb {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let params = BTreeMap::from([
            ("market", req["symbol"].as_str().unwrap_or_default()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("ord_type", req["order_type"].as_str().unwrap_or_default()),
            ("price", req["price"].as_str().unwrap_or_default()),
            ("volume", req["amount"].as_str().unwrap_or_default()),
        ]);

        let query_hash = self.get_query_hash(&params)?;
        let signed_payload = self.get_json(query_hash)?;

        let authorization = format!("Bearer {}", signed_payload);
        let base = self
            .get_end_point_with_key("make_order")
            .ok_or("Endpoint not found".to_string())?;
        
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, "application/json")
            .body(params)
            .map_err(|e| e.to_string())?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let json_value: Value = from_slice(&body).map_err(|e| e.to_string())?;

        Ok(json_value)
    }

    async fn cancel_order(&self, req: Value) -> Result<Value, String> {
        let params = BTreeMap::from([
            ("uuid", req["order_id"].as_str().unwrap_or_default()),
        ]);

        let query_hash = self.get_query_hash(&params)?;
        let signed_payload = self.get_json(query_hash)?;

        let authorization = format!("Bearer {}", signed_payload);
        let base = self
            .get_end_point_with_key("cancel_order")
            .ok_or("Endpoint not found".to_string())?;
        
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, "application/json")
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
            ("markets", symbol.as_str())
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
            .header(ACCEPT, "application/json")
            .body(BTreeMap::new())
            .map_err(|e| e.to_string())?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let json_value: Value = from_slice(&body).map_err(|e| e.to_string())?;

        Ok(json_value)
    }
    
    fn get_name(&self) -> String {
        "Bithumb".to_string()
    }
}

fn parse_symbol(symbol: &str) -> String {
    let v: Vec<&str> = symbol.split("/").collect();
    format!("{}-{}", v[1], v[0])
}



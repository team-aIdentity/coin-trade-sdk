use std::collections::BTreeMap;
use async_trait::async_trait;
use serde_json::{ from_slice, Value };
use http::{ header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE }, Request };
use sha2::{ Sha512, Digest };
use uuid::Uuid;
use hmac::{ Hmac, Mac };
use jwt::SignWithKey;

use crate::{ get_query_string, send, Exchange };

pub struct Upbit {
    api_url: String,
    api_key: String,
    secret: String,
    endpoint: BTreeMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait UpbitTrait {
    fn new(api_key: String, secret: String) -> Result<Self, String> where Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &BTreeMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
    fn send_req_with_sign(
        &self,
        param: BTreeMap<&str, &str>
    ) -> impl std::future::Future<Output = Value> + Send;
}

impl Upbit {
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

impl UpbitTrait for Upbit {
    fn new(api_key: String, secret: String) -> Result<Self, String> {
        Upbit::validate_api_credentials(&api_key, &secret)?;

        let endpoint = BTreeMap::from([
            ("make_order".to_string(), ["POST".to_string(), "v1/orders".to_string()]),
            ("cancel_order".to_string(), ["DELETE".to_string(), "v1/order".to_string()]),
            ("order_book".to_string(), ["GET".to_string(), "v1/orderbook".to_string()]),
        ]);

        Ok(Self {
            api_url: "https://api.upbit.com/".to_string(),
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

    async fn send_req_with_sign(&self, param: BTreeMap<&str, &str>) -> Value {
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

        let key = self.create_hmac_key().unwrap();
        let jwt_token = payload
            .sign_with_key(&key)
            .map_err(|e| e.to_string())
            .unwrap();

        let authorization = format!("Bearer {}", jwt_token);

        let base = self
            .get_end_point_with_key("make_order")
            .ok_or("Endpoint not found".to_string())
            .unwrap();

        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, "application/json")
            .body(param)
            .map_err(|e| e.to_string())
            .unwrap();

        let response = send(request).await
            .map_err(|e| e.to_string())
            .unwrap();
        let body = response.into_body();
        let json_value: Value = from_slice(&body)
            .map_err(|e| e.to_string())
            .unwrap();

        json_value
    }
}

#[async_trait]
impl Exchange for Upbit {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let params = BTreeMap::from([
            ("market", req["symbol"].as_str().unwrap_or_default()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("ord_type", req["order_type"].as_str().unwrap_or_default()),
            ("price", req["price"].as_str().unwrap_or_default()),
            ("volume", req["amount"].as_str().unwrap_or_default()),
        ]);

        Ok(self.send_req_with_sign(params).await)
    }

    async fn cancel_order(&self, req: Value) -> Result<Value, String> {
        let params = BTreeMap::from([("uuid", req["order_id"].as_str().unwrap_or_default())]);

        Ok(self.send_req_with_sign(params).await)
    }

    async fn get_order_book(&self, req: Value) -> Result<Value, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([
            ("markets", symbol.as_str()),
            ("level", "0"),
        ]);

        let query_string = get_query_string(params);

        let base = self
            .get_end_point_with_key("order_book")
            .ok_or("Endpoint not found".to_string())?;

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
        "Upbit".to_string()
    }
}

fn parse_symbol(symbol: &str) -> String {
    let v: Vec<&str> = symbol.split("/").collect();
    format!("{}-{}", v[1], v[0])
}

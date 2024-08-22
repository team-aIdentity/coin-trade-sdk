use std::collections::HashMap;
use async_trait::async_trait;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{from_slice, Value};
use http::Request;
use sha2::{Sha512, Digest};
use uuid::Uuid;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use urlencoding::encode;

use crate::{send, Exchange};

pub struct Upbit {
    api_url: String,
    api_key: String,
    secret: String,
    endpoint: HashMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait UpbitTrait {
    fn new(api_key: String, secret: String) -> Result<Self, String>
    where
        Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &HashMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
    fn get_json(&self, query_hash: String) -> Result<String, String>;
    fn get_query_hash(&self, params: &HashMap<&str, &str>) -> Result<String, String>;
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

        let endpoint = HashMap::from([
            ("make_order".to_string(), ["POST".to_string(), "v1/orders".to_string()]),
            ("cancel_order".to_string(), ["DELETE".to_string(), "v1/order".to_string()])
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

    fn get_end_point(&self) -> &HashMap<String, [String; 2]> {
        &self.endpoint
    }

    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]> {
        self.endpoint.get(key)
    }

    fn get_json(&self, query_hash: String) -> Result<String, String> {
        let nonce = Uuid::new_v4().to_string();
        let payload = HashMap::from([
            ("access_key".to_string(), self.api_key.clone()),
            ("nonce".to_string(), nonce),
            ("query_hash".to_string(), query_hash),
            ("query_hash_alg".to_string(), "SHA512".to_string()),
        ]);

        let key = self.create_hmac_key()?;
        payload
            .sign_with_key(&key)
            .map_err(|e| e.to_string())
    }

    fn get_query_hash(&self, params: &HashMap<&str, &str>) -> Result<String, String> {
        let serialized_params = serde_json::to_string(params).map_err(|e| e.to_string())?;
        let query_string = encode(&serialized_params);

        let mut hasher = Sha512::new();
        hasher.update(query_string.as_bytes());
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }
}

#[async_trait]
impl Exchange for Upbit {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let params = HashMap::from([
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
        let params = HashMap::from([
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

    fn get_name(&self) -> String {
        "Upbit".to_string()
    }
}


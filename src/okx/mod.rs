use std::collections::HashMap;
use async_trait::async_trait;
use http::header::CONTENT_TYPE;
use serde_json::{from_slice, Value};
use http::Request;
use sha2::Sha256;
use hmac::{Hmac, Mac};
use base64::{Engine as _, engine::general_purpose};

use crate::{get_current_timestamp_in_millis, send, Exchange};

pub struct Okx {
    api_url: String,
    api_key: String,
    secret: String,
    passphrase: String,
    endpoint: HashMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait OkxTrait {
    fn get_signature(&self, params: &HashMap<&str, &str>, timestamp: String, method: String, endpoint: String) -> Result<String, String>;
    fn new(api_key: String, secret: String, passphrase: String) -> Result<Self, String>
    where
        Self: Sized;
    fn get_api_url(&self) -> &str;
    fn get_end_point(&self) -> &HashMap<String, [String; 2]>;
    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]>;
}

impl Okx {
    fn validate_api_credentials(api_key: &str, secret: &str, passphrase: &str) -> Result<(), String> {
        if api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if secret.is_empty() {
            return Err("Secret cannot be empty".to_string());
        }
        if passphrase.is_empty() {
            return Err("passphrase cannout be empty".to_string());
        }
        Ok(())
    }

    fn create_hmac_key(&self) -> Result<Hmac<Sha256>, String> {
        Hmac::new_from_slice(self.secret.as_bytes()).map_err(|e| e.to_string())
    }
}

impl OkxTrait for Okx {
    fn new(api_key: String, secret: String, passphrase: String) -> Result<Self, String> {
        Okx::validate_api_credentials(&api_key, &secret, &passphrase)?;

        let endpoint = HashMap::from([
            ("make_order".to_string(), ["POST".to_string(), "api/v5/trade/order".to_string()]),
            ("cancel_order".to_string(), ["POST".to_string(), "api/v5/trade/cancel-order".to_string()])
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

    fn get_end_point(&self) -> &HashMap<String, [String; 2]> {
        &self.endpoint
    }

    fn get_end_point_with_key(&self, key: &str) -> Option<&[String; 2]> {
        self.endpoint.get(key)
    }

    fn get_signature(&self, params: &HashMap<&str, &str>, timestamp: String, method: String, endpoint: String) -> Result<String, String> {
        let query_string = params.iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join("&");

        let mut mac = self.create_hmac_key()?;
        mac.update((timestamp + &method + &endpoint + "?" + &query_string).as_bytes());

        let result = mac.finalize();
        let hmac_bytes = result.into_bytes();
        let b64 = general_purpose::STANDARD.encode(hmac_bytes);

        Ok(hex::encode(b64))
    }
}

#[async_trait]
impl Exchange for Okx {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let get_current_timestamp_in_millis = get_current_timestamp_in_millis().to_string();
        let timestamp = get_current_timestamp_in_millis.as_str();
        let params = HashMap::from([
            ("instId", req["symbol"].as_str().unwrap_or_default()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("ordType", req["order_type"].as_str().unwrap_or_default()),
            ("px", req["price"].as_str().unwrap_or_default()),
            ("sz", req["amount"].as_str().unwrap_or_default()),
            ("tdMode", "cash")
        ]);

        let base = self
            .get_end_point_with_key("make_order")
            .ok_or("Endpoint not found".to_string())?;

        let signature = self.get_signature(
            &params, 
            timestamp.to_string(), 
            base[0].to_string(), 
            base[1].to_string()
        )?;
                
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header("OK-ACCESS-KEY", self.api_key.clone())
            .header("OK-ACCESS-SIGN", &signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", self.passphrase.clone())
            .header(CONTENT_TYPE, "application/json")
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
        let params = HashMap::from([
            ("instId", req["symbol"].as_str().unwrap_or_default()),
            ("ordId", req["order_id"].as_str().unwrap_or_default()),
        ]);

        let base = self
            .get_end_point_with_key("cancel_order")
            .ok_or("Endpoint not found".to_string())?;

        let signature = self.get_signature(
            &params, 
            timestamp.to_string(), 
            base[0].to_string(), 
            base[1].to_string()
        )?;
                
        let request = Request::builder()
            .method(base[0].as_str())
            .uri(format!("{}{}", self.api_url, base[1]))
            .header("OK-ACCESS-KEY", self.api_key.clone())
            .header("OK-ACCESS-SIGN", &signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", self.passphrase.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(params)
            .map_err(|e| e.to_string())?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        let json_value: Value = from_slice(&body).map_err(|e| e.to_string())?;

        Ok(json_value)
    }

    fn get_name(&self) -> String {
        "Okx".to_string()
    }
}


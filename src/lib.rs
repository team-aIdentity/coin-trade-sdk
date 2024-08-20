use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use http::{Request, Version};
use reqwest::{Client, Response};
use serde_json::Value;
use url::Url;

mod binance;
mod bithumb;
mod okx;
mod upbit;

#[async_trait]
pub trait Exchange {
    async fn place_order(&self, req: Value) -> Result<Value, String>;
    async fn cancel_order(&self, symbol: String, order_id: String) -> Result<Value, String>;
}

async fn send(req: Request<HashMap<&str, &str>>) -> Result<http::Response<Vec<u8>> , String> {
    let client = Client::new();
    let uri = req.uri().to_string();
    let url = Url::parse(&uri).unwrap();
    let request = client.request(req.method().clone(), url)
        .headers(req.headers().clone())
        .json(req.body())
        .build().unwrap();
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

    let mut builder = http::Response::builder()
        .status(status)
        .version(version);

    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }

    builder.body(body).expect("Failed to build HTTP response")
}

fn get_current_timestamp_in_millis() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_millis() as u64
}

#[cfg(test)]
mod test;

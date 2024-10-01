use std::collections::BTreeMap;
use async_trait::async_trait;
use serde_json::{ from_slice, Value };
use http::{ header::{ ACCEPT, CONTENT_TYPE }, HeaderName, Request };
use sha2::Sha256;
use hmac::{ Hmac, Mac };
use crate::{
    get_current_timestamp_in_millis,
    get_query_string,
    send,
    CoinList,
    Exchange,
    OrderBook,
    OrderBookUnit,
    Price,
};

pub struct Binance {
    api_url: String,
    api_key: String,
    secret: String,
    endpoint: BTreeMap<String, [String; 2]>,
}

#[allow(dead_code)]
pub trait BinanceTrait {
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

impl Binance {
    fn validate_api_credentials(api_key: &str, secret: &str) -> Result<(), String> {
        if api_key.is_empty() || secret.is_empty() {
            return Err("API key and Secret cannot be empty".to_string());
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
        headers: Vec<(HeaderName, &str)>,
        body: BTreeMap<&'a str, &'a str>
    ) -> Result<Request<BTreeMap<&str, &'a str>>, String> {
        let mut builder = Request::builder().method(method).uri(uri);
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        builder.body(body).map_err(|e| e.to_string())
    }

    fn get_signature(&self, params: &BTreeMap<&str, &str>) -> Result<String, String> {
        let query_string = get_query_string(params.clone());
        let mut mac = self.create_hmac_key()?;
        mac.update(query_string.as_bytes());

        let result = mac.finalize();
        let hmac_bytes = result.into_bytes();
        Ok(hex::encode(hmac_bytes))
    }
}

impl BinanceTrait for Binance {
    fn new(api_key: String, secret: String) -> Result<Self, String> {
        Binance::validate_api_credentials(&api_key, &secret)?;

        let endpoint = BTreeMap::from([
            ("make_order".to_string(), ["POST".to_string(), "api/v3/order".to_string()]),
            ("cancel_order".to_string(), ["DELETE".to_string(), "api/v3/order".to_string()]),
            ("order_book".to_string(), ["GET".to_string(), "api/v3/depth".to_string()]),
            ("current_price".to_string(), ["GET".to_string(), "api/v3/ticker/price".to_string()]),
            ("coin_list".to_string(), ["GET".to_string(), "api/v3/exchangeInfo".to_string()]),
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

    async fn send_req_with_sign(
        &self,
        mut param: BTreeMap<&str, &str>,
        endpoint_key: &str
    ) -> Result<Value, String> {
        let base = self
            .get_end_point_with_key(endpoint_key)
            .ok_or("Endpoint not found".to_string())?;

        let uri = format!("{}{}", self.api_url, base[1]);
        let signature = self.get_signature(&param)?;

        param.insert("signature", &signature);
        let request = self.build_request(
            base[0].as_str(),
            &uri,
            vec![
                (CONTENT_TYPE, "application/x-www-form-urlencoded"),
                ("X-MBX-APIKEY".try_into().unwrap(), self.api_key.as_str())
            ],
            param
        )?;

        let response = send(request).await.map_err(|e| e.to_string())?;
        let body = response.into_body();
        from_slice(&body).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Exchange for Binance {
    async fn place_order(&self, req: Value) -> Result<Value, String> {
        let timestamp_ = get_current_timestamp_in_millis().to_string();
        let symbol = parse_symbol(req["symbol"].as_str().unwrap_or_default());
        let params = BTreeMap::from([
            ("symbol", symbol.as_str()),
            ("side", req["side"].as_str().unwrap_or_default()),
            ("type", req["order_type"].as_str().unwrap_or_default()),
            ("price", req["price"].as_str().unwrap_or_default()),
            ("quantity", req["amount"].as_str().unwrap_or_default()),
            ("timestamp", &timestamp_),
            ("newOrderRespType", "RESULT"),
        ]);

        self.send_req_with_sign(params, "make_order").await
    }

    async fn cancel_order(&self, req: Value) -> Result<Value, String> {
        let timestamp_ = get_current_timestamp_in_millis().to_string();
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([
            ("symbol", symbol.as_str()),
            ("orderId", req["order_id"].as_str().unwrap_or_default()),
            ("timestamp", &timestamp_),
        ]);

        self.send_req_with_sign(params, "cancel_order").await
    }

    async fn get_order_book(&self, req: Value) -> Result<OrderBook, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([("symbol", symbol.as_str())]);

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
        Ok(parse_orderbook(res, req["symbol"].as_str().unwrap().to_string())?)
    }

    fn get_name(&self) -> String {
        "Binance".to_string()
    }

    async fn get_current_price(&self, req: Value) -> Result<Price, String> {
        let symbol = parse_symbol(req["symbol"].as_str().unwrap());
        let params = BTreeMap::from([("symbol", symbol.as_str())]);

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
        let res: Value = from_slice(&body).map_err(|e| e.to_string())?;

        println!(">>>>>>>>>>>>>>>>>>>>> {:?}", res);

        // Parsing response to create Price struct
        let symbol_name = req["symbol"].as_str().unwrap().to_string();
        let current_price = res["price"].as_str().unwrap_or("0.0").to_string();

        let price = Price {
            exchange: "Binance".to_string(),
            symbol: symbol_name,
            price: current_price,
        };

        Ok(price)
    }

    async fn get_coin_list(&self) -> Result<CoinList, String> {
        let params = BTreeMap::from([("permissions", "SPOT")]);

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
        let market = "Binance".to_string();
        let coin_list = res["symbols"]
            .as_array()
            .ok_or("Response is not an array".to_string())?
            .iter()
            .filter_map(|coin|
                format!(
                    "{}/{}",
                    coin["baseAsset"].as_str().unwrap(),
                    coin["quoteAsset"].as_str().unwrap()
                ).into()
            )
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
    format!("{}{}", v[0], v[1])
}

fn parse_orderbook(orderbook_res: Value, symbol: String) -> Result<OrderBook, String> {
    // Extract asks and bids from the response
    let asks = orderbook_res["asks"].as_array().ok_or("Asks field is not an array")?;
    let bids = orderbook_res["bids"].as_array().ok_or("Bids field is not an array")?;

    // Ensure the lengths of asks and bids are the same
    let mut orderbook_units = Vec::new();
    let len = asks.len().min(bids.len());

    for i in 0..len {
        // Parse ask and bid for each level
        let ask = &asks[i];
        let bid = &bids[i];

        // Ensure both ask and bid are arrays of size 2
        let ask_price = ask[0].as_str().unwrap_or_default().to_string();
        let ask_size = ask[1].as_str().unwrap_or_default().to_string();
        let bid_price = bid[0].as_str().unwrap_or_default().to_string();
        let bid_size = bid[1].as_str().unwrap_or_default().to_string();

        // Push the parsed values into orderbook_units
        orderbook_units.push(OrderBookUnit {
            ask_price,
            bid_price,
            ask_size,
            bid_size,
        });
    }

    // Create and return the OrderBook struct
    Ok(OrderBook {
        market: symbol,
        exchange: "Binance".to_string(),
        orderbook_unit: orderbook_units,
    })
}

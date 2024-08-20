use std::collections::HashMap;

use crate::binance::{Binance, BinanceTrait};

// 헬퍼 함수: Binance 객체 생성
fn create_test_binance() -> Binance {
    Binance::new("test_api_key".to_string(), "test_secret".to_string()).unwrap()
}

// 헬퍼 함수: 에러 메시지 검증
fn assert_binance_creation_error(api_key: &str, secret: &str, expected_error: &str) {
    let result = Binance::new(api_key.to_string(), secret.to_string());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), expected_error.to_string());
}

#[test]
fn test_new_binance_with_valid_credentials() {
    let binance = create_test_binance();
    assert_eq!(binance.get_api_url(), "https://api.binance.com/");
}

#[test]
fn test_new_binance_with_empty_api_key() {
    assert_binance_creation_error("", "test_secret", "API key cannot be empty");
}

#[test]
fn test_new_binance_with_empty_secret() {
    assert_binance_creation_error("test_api_key", "", "Secret cannot be empty");
}

#[test]
fn test_new_binance_with_empty_credentials() {
    assert_binance_creation_error("", "", "API key cannot be empty");
}

#[test]
fn test_get_end_point() {
    let binance = create_test_binance();
    let endpoints = binance.get_end_point();
    let expected_endpoints = HashMap::from([
        ("make_order".to_string(), ["POST".to_string(), "api/v3/order".to_string()]),
        ("cancel_order".to_string(), ["DELETE".to_string(), "api/v3/order".to_string()]),
    ]);

    assert_eq!(endpoints, &expected_endpoints);
}

#[test]
fn test_get_end_point_with_key_existing() {
    let binance = create_test_binance();
    let endpoint = binance.get_end_point_with_key("make_order");
    assert!(endpoint.is_some());
    assert_eq!(endpoint.unwrap(), &["POST".to_string(), "api/v3/order".to_string()]);

    let endpoint = binance.get_end_point_with_key("cancel_order");
    assert!(endpoint.is_some());
    assert_eq!(endpoint.unwrap(), &["DELETE".to_string(), "api/v3/order".to_string()]);
}

#[test]
fn test_get_end_point_with_key_non_existing() {
    let binance = create_test_binance();
    let endpoint = binance.get_end_point_with_key("non_existing");
    assert!(endpoint.is_none());
}

#[test]
fn test_get_signature() {
    let binance = create_test_binance();
    let params = HashMap::from([
        ("symbol", "BTCUSDT"),
        ("side", "BUY"),
        ("type", "LIMIT"),
        ("price", "50000"),
        ("quantity", "0.01"),
        ("timestamp", "1622547800"),
    ]);

    let signature = binance.get_signature(&params);
    assert!(signature.is_ok());
    // 정확한 해시값을 테스트하기 위해 적절한 검증 코드를 추가할 수 있습니다.
}


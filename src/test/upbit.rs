use std::collections::BTreeMap;
use crate::upbit::{Upbit, UpbitTrait};

// 헬퍼 함수: Upbit 객체 생성
fn create_test_upbit() -> Upbit {
    Upbit::new("test_api_key".to_string(), "test_secret".to_string()).unwrap()
}

// 헬퍼 함수: 에러 메시지 검증
fn assert_upbit_creation_error(api_key: &str, secret: &str, expected_error: &str) {
    let result = Upbit::new(api_key.to_string(), secret.to_string());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), expected_error.to_string());
}

#[test]
fn test_new_upbit_with_valid_credentials() {
    let upbit = create_test_upbit();
    assert_eq!(upbit.get_api_url(), "https://api.upbit.com/");
}

#[test]
fn test_new_upbit_with_empty_api_key() {
    assert_upbit_creation_error("", "test_secret", "API key cannot be empty");
}

#[test]
fn test_new_upbit_with_empty_secret() {
    assert_upbit_creation_error("test_api_key", "", "Secret cannot be empty");
}

#[test]
fn test_new_upbit_with_empty_credentials() {
    assert_upbit_creation_error("", "", "API key cannot be empty");
}

#[test]
fn test_get_end_point() {
    let upbit = create_test_upbit();
    let endpoints = upbit.get_end_point();
    let expected_endpoints = BTreeMap::from([
        ("make_order".to_string(), ["POST".to_string(), "v1/orders".to_string()]),
        ("cancel_order".to_string(), ["DELETE".to_string(), "v1/order".to_string()])
    ]);

    assert_eq!(endpoints, &expected_endpoints);
}

#[test]
fn test_get_end_point_with_key_existing() {
    let upbit = create_test_upbit();
    let endpoint = upbit.get_end_point_with_key("make_order");
    assert!(endpoint.is_some());
    assert_eq!(endpoint.unwrap(), &["POST".to_string(), "v1/orders".to_string()]);

    let endpoint = upbit.get_end_point_with_key("cancel_order");
    assert!(endpoint.is_some());
    assert_eq!(endpoint.unwrap(), &["DELETE".to_string(), "v1/order".to_string()]);

}

#[test]
fn test_get_end_point_with_key_non_existing() {
    let upbit = create_test_upbit();
    let endpoint = upbit.get_end_point_with_key("non_existing");
    assert!(endpoint.is_none());
}

#[test]
fn test_get_query_hash() {
    let upbit = create_test_upbit();
    let params = BTreeMap::from([
        ("market", "BTC-USD"),
        ("side", "buy"),
        ("ord_type", "limit"),
        ("price", "50000"),
        ("volume", "0.01"),
    ]);

    let query_hash = upbit.get_query_hash(&params);
    assert!(query_hash.is_ok());
    // 해시값이 정확한지 확인하는 부분은 테스트 환경에 맞게 추가할 수 있습니다.
}

#[test]
fn test_get_json_with_valid_query_hash() {
    let upbit = create_test_upbit();
    let query_hash = "valid_query_hash".to_string();
    let json_result = upbit.get_json(query_hash);
    assert!(json_result.is_ok());
    // 결과가 유효한지 추가로 확인하는 부분은 테스트 환경에 맞게 구현할 수 있습니다.
}


use std::collections::HashMap;
use crate::okx::{Okx, OkxTrait};

// Helper function: Create a test Okx object
fn create_test_okx() -> Okx {
    Okx::new("test_api_key".to_string(), "test_secret".to_string(), "test_passphrase".to_string()).unwrap()
}

// Helper function: Assert error on Okx creation
fn assert_okx_creation_error(api_key: &str, secret: &str, passphrase: &str, expected_error: &str) {
    let result = Okx::new(api_key.to_string(), secret.to_string(), passphrase.to_string());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), expected_error.to_string());
}

#[test]
fn test_new_okx_with_valid_credentials() {
    let okx = create_test_okx();
    assert_eq!(okx.get_api_url(), "https://api.okx.com/");
}

#[test]
fn test_new_okx_with_empty_api_key() {
    assert_okx_creation_error("", "test_secret", "test_passphrase", "API key cannot be empty");
}

#[test]
fn test_new_okx_with_empty_secret() {
    assert_okx_creation_error("test_api_key", "", "test_passphrase", "Secret cannot be empty");
}

#[test]
fn test_new_okx_with_empty_passphrase() {
    assert_okx_creation_error("test_api_key", "test_secret", "", "passphrase cannout be empty");
}

#[test]
fn test_new_okx_with_empty_credentials() {
    assert_okx_creation_error("", "", "", "API key cannot be empty");
}

#[test]
fn test_get_end_point() {
    let okx = create_test_okx();
    let endpoints = okx.get_end_point();
    let expected_endpoints = HashMap::from([
        ("make_order".to_string(), ["POST".to_string(), "api/v5/trade/order".to_string()]),
        ("cancel_order".to_string(), ["DELETE".to_string(), "api/v5/trade/cancel-order".to_string()])
    ]);

    assert_eq!(endpoints, &expected_endpoints);
}

#[test]
fn test_get_end_point_with_key_existing() {
    let okx = create_test_okx();
    let endpoint = okx.get_end_point_with_key("make_order");
    assert!(endpoint.is_some());
    assert_eq!(endpoint.unwrap(), &["POST".to_string(), "api/v5/trade/order".to_string()]);

    let endpoint = okx.get_end_point_with_key("cancel_order");
    assert!(endpoint.is_some());
    assert_eq!(endpoint.unwrap(), &["DELETE".to_string(), "api/v5/trade/cancel-order".to_string()]);
}

#[test]
fn test_get_end_point_with_key_non_existing() {
    let okx = create_test_okx();
    let endpoint = okx.get_end_point_with_key("non_existing");
    assert!(endpoint.is_none());
}

#[test]
fn test_get_signature() {
    let okx = create_test_okx();
    let params = HashMap::from([
        ("instId", "BTC-USDT"),
        ("side", "buy"),
        ("ordType", "limit"),
        ("px", "50000"),
        ("sz", "0.01")
    ]);

    let timestamp = "2024-08-20T00:00:00Z".to_string();
    let method = "POST".to_string();
    let endpoint = "api/v5/trade/order".to_string();

    let signature = okx.get_signature(&params, timestamp.clone(), method.clone(), endpoint.clone());
    assert!(signature.is_ok());
    // Note: Actual value of the signature would depend on the HMAC calculation
    // In a real test, you might want to compare it with a known correct value
    // For this example, we just check if it's not an error.
}



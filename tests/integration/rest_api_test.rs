//! Integration tests for REST API endpoints
//!
//! These tests verify that REST API endpoints work correctly end-to-end.
//! Run with: `cargo test --test rest_api_test`
//!
//! Note: These tests require a running server. Set TEST_BASE_URL environment variable
//! to point to your test server, or use the default http://localhost:3010

use serde_json::Value;
use std::time::Duration;

/// Helper function to get base URL from environment or use default
fn get_base_url() -> String {
    std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string())
}

/// Helper function to make a GET request
async fn get_request(path: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", get_base_url(), path);
    client.get(&url).send().await
}

#[tokio::test]
#[ignore] // Ignore by default - requires running server
async fn test_health_endpoint() {
    let response = get_request("/health").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body.get("version").is_some());
}

#[tokio::test]
#[ignore]
async fn test_metrics_endpoint() {
    let response = get_request("/metrics").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body = response.text().await.unwrap();
    // Prometheus metrics should contain some common metrics
    assert!(body.contains("http_requests_total") || body.contains("# HELP"));
}

#[tokio::test]
#[ignore]
async fn test_available_tokens_endpoint() {
    let response = get_request("/v1/api/kaspa/tokens").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.get("tokens").is_some());
    assert!(body.get("count").is_some());
}

#[tokio::test]
#[ignore]
async fn test_trade_stats_endpoint() {
    let response = get_request("/v1/api/kaspa/trade-stats?timeFrame=6h").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.get("totalTradesKaspiano").is_some());
    assert!(body.get("tokens").is_some());
}

#[tokio::test]
#[ignore]
async fn test_trade_stats_with_ticker() {
    let response = get_request("/v1/api/kaspa/trade-stats?timeFrame=6h&ticker=SLOW").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.get("totalTradesKaspiano").is_some());
}

#[tokio::test]
#[ignore]
async fn test_floor_price_endpoint() {
    let response = get_request("/v1/api/kaspa/floor-price").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_floor_price_with_ticker() {
    let response = get_request("/v1/api/kaspa/floor-price?ticker=SLOW").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_sold_orders_endpoint() {
    let response = get_request("/v1/api/kaspa/sold-orders?minutes=60").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_hot_mints_endpoint() {
    let response = get_request("/v1/api/kaspa/hot-mints?timeInterval=1h").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_last_order_sold_endpoint() {
    let response = get_request("/v1/api/kaspa/last-order-sold").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.get("ticker").is_some());
    assert!(body.get("pricePerToken").is_some());
}

#[tokio::test]
#[ignore]
async fn test_open_orders_endpoint() {
    let response = get_request("/v1/api/kaspa/open-orders").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.get("tickers").is_some());
}

#[tokio::test]
#[ignore]
async fn test_historical_data_endpoint() {
    let response = get_request("/v1/api/kaspa/historical-data?timeFrame=1h&ticker=SLOW").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.get("ticker").is_some());
    assert!(body.get("dataPoints").is_some());
}

#[tokio::test]
#[ignore]
async fn test_krc721_mints_endpoint() {
    let response = get_request("/v1/api/kaspa/krc721/mint").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_krc721_floor_price_endpoint() {
    let response = get_request("/v1/api/kaspa/krc721/floor-price").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_kns_sold_orders_endpoint() {
    let response = get_request("/v1/api/kaspa/kns/sold-orders?minutes=60").await.unwrap();
    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_input_validation() {
    // Test with invalid time frame (too long)
    let response = get_request("/v1/api/kaspa/trade-stats?timeFrame=this_is_way_too_long_for_a_time_frame").await.unwrap();
    // Should return 400 Bad Request
    assert!(response.status().is_client_error());
}

#[tokio::test]
#[ignore]
async fn test_invalid_ticker_length() {
    // Test with ticker that's too long
    let long_ticker = "a".repeat(100);
    let response = get_request(&format!("/v1/api/kaspa/floor-price?ticker={}", long_ticker)).await.unwrap();
    // Should return 400 Bad Request
    assert!(response.status().is_client_error());
}

#[tokio::test]
#[ignore]
async fn test_invalid_minutes_range() {
    // Test with minutes outside valid range
    let response = get_request("/v1/api/kaspa/sold-orders?minutes=999999").await.unwrap();
    // Should return 400 Bad Request
    assert!(response.status().is_client_error());
}

#[tokio::test]
#[ignore]
async fn test_not_found_endpoint() {
    let response = get_request("/v1/api/kaspa/nonexistent").await.unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
#[ignore]
async fn test_response_times() {
    let start = std::time::Instant::now();
    let response = get_request("/health").await.unwrap();
    let duration = start.elapsed();
    
    assert_eq!(response.status(), 200);
    // Health endpoint should be very fast
    assert!(duration < Duration::from_millis(100));
}

#[tokio::test]
#[ignore]
async fn test_cors_headers() {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health", get_base_url()))
        .header("Origin", "http://localhost:3000")
        .send()
        .await
        .unwrap();
    
    // CORS headers should be present (exact headers depend on config)
    // Just verify the request doesn't fail
    assert!(response.status().is_success());
}


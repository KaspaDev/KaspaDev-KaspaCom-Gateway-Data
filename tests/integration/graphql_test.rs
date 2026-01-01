//! Integration tests for GraphQL API endpoints
//!
//! These tests verify that the GraphQL API works correctly end-to-end.
//! Run with: `cargo test --test graphql_test`

use serde_json::{json, Value};
use std::time::Duration;

/// Helper function to make a GraphQL request
async fn graphql_query(client: &reqwest::Client, base_url: &str, query: &str) -> Result<Value, reqwest::Error> {
    let response = client
        .post(&format!("{}/graphql", base_url))
        .json(&json!({
            "query": query
        }))
        .send()
        .await?;
    
    response.json().await
}

/// Helper function to make a GraphQL request with variables
async fn graphql_query_with_vars(
    client: &reqwest::Client,
    base_url: &str,
    query: &str,
    variables: Value,
) -> Result<Value, reqwest::Error> {
    let response = client
        .post(&format!("{}/graphql", base_url))
        .json(&json!({
            "query": query,
            "variables": variables
        }))
        .send()
        .await?;
    
    response.json().await
}

#[tokio::test]
#[ignore] // Ignore by default - requires running server
async fn test_graphql_health_check() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    // Test that GraphQL endpoint is accessible
    let response = client
        .get(&format!("{}/graphql", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[ignore]
async fn test_krc20_floor_prices_all() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let query = r#"
        query {
            krc20FloorPrices {
                ticker
                floorPrice
                volume
            }
        }
    "#;
    
    let response = graphql_query(&client, &base_url, query).await.unwrap();
    
    // Should not have errors
    assert!(!response.get("errors").is_some(), "Query should not have errors: {:?}", response);
    
    // Should have data
    let data = response.get("data").expect("Response should have data field");
    let floor_prices = data.get("krc20FloorPrices").expect("Should have krc20FloorPrices field");
    
    assert!(floor_prices.is_array(), "krc20FloorPrices should be an array");
    
    // If array is not empty, verify structure
    if let Some(first) = floor_prices.as_array().and_then(|arr| arr.first()) {
        assert!(first.get("ticker").is_some(), "Floor price should have ticker");
        assert!(first.get("floorPrice").is_some(), "Floor price should have floorPrice");
    }
}

#[tokio::test]
#[ignore]
async fn test_krc20_floor_prices_with_ticker() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let query = r#"
        query {
            krc20FloorPrices(ticker: "SLOW") {
                ticker
                floorPrice
            }
        }
    "#;
    
    let response = graphql_query(&client, &base_url, query).await.unwrap();
    
    assert!(!response.get("errors").is_some(), "Query should not have errors");
    
    let data = response.get("data").expect("Response should have data field");
    let floor_prices = data.get("krc20FloorPrices").expect("Should have krc20FloorPrices field");
    
    if let Some(arr) = floor_prices.as_array() {
        // If we got results, verify they match the ticker
        for item in arr {
            if let Some(ticker) = item.get("ticker").and_then(|t| t.as_str()) {
                assert_eq!(ticker, "SLOW", "All results should be for SLOW ticker");
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_graphql_with_variables() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let query = r#"
        query GetFloorPrices($ticker: String) {
            krc20FloorPrices(ticker: $ticker) {
                ticker
                floorPrice
            }
        }
    "#;
    
    let variables = json!({
        "ticker": "SLOW"
    });
    
    let response = graphql_query_with_vars(&client, &base_url, query, variables).await.unwrap();
    
    assert!(!response.get("errors").is_some(), "Query should not have errors");
    assert!(response.get("data").is_some(), "Response should have data field");
}

#[tokio::test]
#[ignore]
async fn test_graphql_invalid_query() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let query = r#"
        query {
            invalidField {
                data
            }
        }
    "#;
    
    let response = graphql_query(&client, &base_url, query).await.unwrap();
    
    // Should have errors
    assert!(response.get("errors").is_some(), "Invalid query should return errors");
    
    let errors = response.get("errors").unwrap().as_array().unwrap();
    assert!(!errors.is_empty(), "Should have at least one error");
}

#[tokio::test]
#[ignore]
async fn test_graphql_schema_introspection() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let query = r#"
        query {
            __schema {
                queryType {
                    name
                }
                types {
                    name
                }
            }
        }
    "#;
    
    let response = graphql_query(&client, &base_url, query).await.unwrap();
    
    assert!(!response.get("errors").is_some(), "Introspection should not have errors");
    
    let data = response.get("data").expect("Response should have data field");
    let schema = data.get("__schema").expect("Should have __schema field");
    let query_type = schema.get("queryType").expect("Should have queryType field");
    
    assert_eq!(query_type.get("name").and_then(|n| n.as_str()), Some("Query"));
}

#[tokio::test]
#[ignore]
async fn test_graphql_performance() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let query = r#"
        query {
            krc20FloorPrices {
                ticker
                floorPrice
            }
        }
    "#;
    
    let start = std::time::Instant::now();
    let response = graphql_query(&client, &base_url, query).await.unwrap();
    let duration = start.elapsed();
    
    assert!(!response.get("errors").is_some(), "Query should not have errors");
    
    // Query should complete in reasonable time (< 1 second for cached data)
    assert!(
        duration < Duration::from_secs(1),
        "Query took too long: {:?}",
        duration
    );
}

#[tokio::test]
#[ignore]
async fn test_graphql_error_handling() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    // Test with malformed JSON
    let response = client
        .post(&format!("{}/graphql", base_url))
        .body("invalid json")
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();
    
    // Should return 400 or 500
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Malformed request should return error status"
    );
}

#[tokio::test]
#[ignore]
async fn test_graphql_query_size_limit() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    // Create a query that's too large (>50KB)
    let large_query = format!("query {{ krc20FloorPrices {{ ticker {} }} }}", "floorPrice ".repeat(10000));
    
    let response = client
        .post(&format!("{}/graphql", base_url))
        .json(&serde_json::json!({
            "query": large_query
        }))
        .send()
        .await
        .unwrap();
    
    let data: serde_json::Value = response.json().await.unwrap();
    
    // Should have an error about query being too large
    assert!(data.get("errors").is_some(), "Should return error for oversized query");
    let errors = data.get("errors").unwrap().as_array().unwrap();
    assert!(!errors.is_empty());
    
    // Check error message contains size limit info
    let error_msg = errors[0].get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(error_msg.contains("too large") || error_msg.contains("QUERY_TOO_LARGE"));
}

#[tokio::test]
#[ignore]
async fn test_graphql_empty_query() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    let response = client
        .post(&format!("{}/graphql", base_url))
        .json(&serde_json::json!({
            "query": ""
        }))
        .send()
        .await
        .unwrap();
    
    let data: serde_json::Value = response.json().await.unwrap();
    
    // Should have an error about empty query
    assert!(data.get("errors").is_some());
    let errors = data.get("errors").unwrap().as_array().unwrap();
    assert!(!errors.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_graphql_all_queries() {
    let client = reqwest::Client::new();
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    
    // Test all major queries exist and are callable
    let queries = vec![
        ("krc20FloorPrices", r#"query { krc20FloorPrices { ticker floorPrice } }"#),
        ("tradeStats", r#"query { tradeStats { totalTradesKaspiano } }"#),
        ("soldOrders", r#"query { soldOrders { ticker } }"#),
        ("hotMints", r#"query { hotMints { ticker } }"#),
        ("openOrders", r#"query { openOrders { tickers } }"#),
        ("krc721Mints", r#"query { krc721Mints { ticker } }"#),
        ("krc721FloorPrices", r#"query { krc721FloorPrices { ticker floorPrice } }"#),
        ("knsSoldOrders", r#"query { knsSoldOrders { assetId } }"#),
    ];
    
    for (name, query) in queries {
        let response = graphql_query(&client, &base_url, query).await;
        
        match response {
            Ok(data) => {
                // Query should not have syntax errors
                if let Some(errors) = data.get("errors") {
                    if errors.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
                        // Some queries might fail due to missing data, but shouldn't have syntax errors
                        let error_msg = errors[0].get("message").and_then(|m| m.as_str()).unwrap_or("");
                        assert!(
                            !error_msg.contains("Cannot query field") && !error_msg.contains("Unknown field"),
                            "Query {} should not have syntax errors: {:?}",
                            name,
                            error_msg
                        );
                    }
                }
            }
            Err(e) => {
                panic!("Query {} failed with network error: {}", name, e);
            }
        }
    }
}


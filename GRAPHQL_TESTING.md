# GraphQL Testing Quick Reference

## Quick Start

### 1. Start the Server

```bash
cargo run --release
# or
docker compose up
```

### 2. Open GraphiQL Playground

Navigate to: **http://localhost:8088/graphql** (development) or **http://localhost:8080/graphql** (production)

### 3. Run Automated Tests

**Bash Script:**

```bash
./tests/graphql_test.sh
# or with custom URL:
./tests/graphql_test.sh http://localhost:8088
```

**Rust Integration Tests:**

```bash
# Run all integration tests
cargo test --test graphql_test

# Run specific test
cargo test --test graphql_test test_krc20_floor_prices_all

# Run with server URL
TEST_BASE_URL=http://localhost:8088 cargo test --test graphql_test
```

## Example Queries

### Get All Floor Prices

```graphql
query {
  krc20FloorPrices {
    ticker
    floorPrice
  }
}
```

### Get Floor Prices for Specific Ticker

```graphql
query {
  krc20FloorPrices(ticker: "SLOW") {
    ticker
    floorPrice
  }
}
```

### Using Variables

```graphql
query GetFloorPrices($ticker: String) {
  krc20FloorPrices(ticker: $ticker) {
    ticker
    floorPrice
  }
}
```

**Variables:**

```json
{
  "ticker": "SLOW"
}
```

## Testing with cURL

### Basic Query

```bash
curl -X POST http://localhost:8088/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { krc20FloorPrices { ticker floorPrice } }"
  }'
```

### Query with Variables

```bash
curl -X POST http://localhost:8088/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetPrices($ticker: String) { krc20FloorPrices(ticker: $ticker) { ticker } }",
    "variables": { "ticker": "SLOW" }
  }'
```

### Schema Introspection

```bash
curl -X POST http://localhost:8088/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { __schema { queryType { name fields { name } } } }"
  }'
```

## Testing with JavaScript/TypeScript

### Using fetch

```javascript
const response = await fetch("http://localhost:8088/graphql", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    query: `
      query {
        krc20FloorPrices(ticker: "SLOW") {
          ticker
          floorPrice
        }
      }
    `,
  }),
});

const data = await response.json();
console.log(data);
```

### Using graphql-request

```bash
npm install graphql-request graphql
```

```typescript
import { request, gql } from "graphql-request";

const query = gql`
  query GetFloorPrices($ticker: String) {
    krc20FloorPrices(ticker: $ticker) {
      ticker
      floorPrice
    }
  }
`;

const data = await request("http://localhost:8088/graphql", query, {
  ticker: "SLOW",
});
```

## Expected Response Format

### Success Response

```json
{
  "data": {
    "krc20FloorPrices": [
      {
        "ticker": "SLOW",
        "floorPrice": 0.00015
      }
    ]
  }
}
```

### Error Response

```json
{
  "errors": [
    {
      "message": "Cannot query field \"invalidField\" on type \"Query\"",
      "locations": [
        {
          "line": 1,
          "column": 9
        }
      ]
    }
  ],
  "data": null
}
```

## Troubleshooting

### "Connection refused"

- Ensure server is running: `cargo run` or `docker compose up`
- Check port: default is `8088` for development (./start.sh) or `8080` for production (config.yaml)

### "Query field not found"

- Check schema: use introspection query
- Verify field name matches exactly (case-sensitive)

### "Timeout"

- Check server logs for errors
- Verify Redis/cache is accessible
- Check rate limits

### "Empty results"

- Verify data exists in cache/API
- Check ticker name (case-sensitive)
- Check server logs for errors

## Performance Testing

### Using Apache Bench

```bash
# Create query.json file
echo '{"query": "query { krc20FloorPrices { ticker floorPrice } }"}' > query.json

# Run benchmark
ab -n 1000 -c 10 -p query.json -T application/json \
  http://localhost:8088/graphql
```

### Using wrk

```bash
# Create query.lua
cat > query.lua << 'EOF'
wrk.method = "POST"
wrk.body = '{"query": "query { krc20FloorPrices { ticker } }"}'
wrk.headers["Content-Type"] = "application/json"
EOF

# Run benchmark
wrk -t4 -c100 -d30s -s query.lua http://localhost:8088/graphql
```

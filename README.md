# 📊 KaspaCom Data API Gateway

> Production-ready REST and GraphQL API gateway for accessing Kaspa.com L1 Marketplace data (KRC20 tokens, KRC721 NFTs, KNS domains) with tiered caching, rate limiting, and comprehensive observability.

[![API Status](https://img.shields.io/badge/API-Live-brightgreen)](http://localhost:8080/health)
[![Swagger](https://img.shields.io/badge/Docs-Swagger%20UI-orange)](http://localhost:8080/swagger-ui)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## ✨ Features

- 🚀 **Fast**: Tiered caching (Redis + Parquet) with <50ms cached response times
- 📈 **Real-time**: Data from Kaspa.com L1 Marketplace API
- 🔧 **Simple API**: REST endpoints with intuitive paths
- 📊 **GraphQL API**: 20+ flexible queries with query optimization
- ⚡ **High Performance**: >90% cache hit rate target, sub-10ms Redis access
- 🛡️ **Secure**: Input validation, rate limiting, security headers
- 📊 **Observable**: Prometheus metrics, structured logging, request tracing
- 🔄 **Marketplace Data**: KRC20 tokens, KRC721 NFTs, KNS domains

---

## 🚀 Quick Start

### 1. Try GraphQL
```bash
# Open GraphiQL playground
open http://localhost:3010/graphql

# Or query via cURL
curl -X POST http://localhost:3010/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "query { krc20FloorPrices { ticker floorPrice } }"}'
```

### 2. Get KRC20 Token Data
```bash
# Get floor prices for all tokens
curl http://localhost:8080/v1/api/kaspa/krc20/floor-prices

# Get trade statistics
curl "http://localhost:8080/v1/api/kaspa/krc20/trade-stats?timeFrame=6h&ticker=SLOW"
```

### 3. Explore the API
Open the interactive docs at: **http://localhost:8080/swagger-ui**

### 4. Check Cache Performance
```bash
# View cache statistics and hit rates
curl http://localhost:3010/v1/api/kaspa/cache/stats
```

---

## 📡 API Reference

### GraphQL API *(Flexible Queries)*

Flexible query interface for fetching exactly the data you need. GraphQL allows you to request only the fields you need, reducing payload size and improving performance.

**GraphiQL Playground**: http://localhost:3010/graphql

**Features:**
- ✅ 20+ queries covering KRC20, KRC721, and KNS data
- ✅ Query complexity analysis (max: 1000)
- ✅ Query depth limiting (max: 10 levels)
- ✅ Query size limits (max: 50KB)
- ✅ Automatic caching with tiered storage
- ✅ Real-time error handling and logging

**Available Queries:**

| Category | Queries |
|----------|---------|
| **KRC20 Tokens** | `tradeStats`, `krc20FloorPrices`, `soldOrders`, `lastOrderSold`, `hotMints`, `tokenInfo`, `tokenLogos`, `openOrders`, `historicalData` |
| **KRC721 NFTs** | `krc721Mints`, `krc721SoldOrders`, `krc721ListedOrders`, `krc721TradeStats`, `krc721HotMints`, `krc721FloorPrices`, `krc721CollectionInfo`, `nftMetadata` |
| **KNS Domains** | `knsSoldOrders`, `knsTradeStats`, `knsListedOrders` |

**Example Queries:**

```graphql
# Get floor prices for all tokens
query {
  krc20FloorPrices {
    ticker
    floorPrice
  }
}

# Get floor prices for specific token
query {
  krc20FloorPrices(ticker: "SLOW") {
    ticker
    floorPrice
  }
}

# Get trade statistics
query {
  tradeStats(timeFrame: "6h", ticker: "SLOW") {
    totalTradesKaspiano
    totalVolumeKasKaspiano
    tokens {
      ticker
      totalTrades
      totalVolumeKas
    }
  }
}

# Get sold orders
query {
  soldOrders(ticker: "SLOW", minutes: 60) {
    id
    ticker
    amount
    pricePerToken
    totalPrice
    sellerAddress
    buyerAddress
    createdAt
  }
}

# Get hot minting tokens
query {
  hotMints(timeInterval: "1h") {
    ticker
    changeTotalMints
    totalMintPercentage
    totalHolders
  }
}

# Get token information
query {
  tokenInfo(ticker: "SLOW") {
    ticker
    totalSupply
    totalMinted
    totalHolders
    price
    marketCap
    volumeUsd
  }
}

# Get NFT mints
query {
  krc721Mints(ticker: "BITCOIN") {
    ticker
    tokenId
    minterAddress
    timestamp
  }
}

# Get NFT metadata
query {
  nftMetadata(ticker: "BITCOIN", tokenId: 1) {
    image
    name
    description
    attributes {
      traitType
      value
    }
  }
}

# Get KRC721 collection information
query {
  krc721CollectionInfo(ticker: "BITCOIN") {
    ticker
    totalSupply
    totalMinted
    totalHolders
    price
  }
}

# Get KNS sold orders
query {
  knsSoldOrders(minutes: 60) {
    assetId
    price
    sellerAddress
    buyerAddress
    createdAt
  }
}

# Get KNS trade statistics
query {
  knsTradeStats(timeFrame: "6h") {
    totalTradesKaspiano
    totalVolumeKasKaspiano
    totalVolumeUsdKaspiano
  }
}

# Get historical price data
query {
  historicalData(timeFrame: "1h", ticker: "SLOW") {
    timeFrame
    bucketSize
    ticker
    dataPoints {
      timestamp
      totalVolumeKas
      averagePrice
      tradeCount
      ticker
    }
    totalDataPoints
  }
}
```

**Using cURL:**
```bash
curl -X POST http://localhost:3010/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { krc20FloorPrices { ticker floorPrice } }"
  }'
```

**Using Variables:**
```bash
curl -X POST http://localhost:3010/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetPrices($ticker: String) { krc20FloorPrices(ticker: $ticker) { ticker floorPrice } }",
    "variables": { "ticker": "SLOW" }
  }'
```

**Query Limits & Validation:**
- Maximum query size: **50KB**
- Maximum query depth: **10 levels**
- Maximum query complexity: **1000**
- Empty queries are rejected
- All queries are automatically cached

**Error Handling:**
GraphQL errors include error codes for easy debugging:
- `QUERY_TOO_LARGE` - Query exceeds 50KB limit
- `EMPTY_QUERY` - Query is empty or whitespace only
- Standard GraphQL validation errors

**Complete Query Reference:**

| Query | Description | Parameters |
|-------|-------------|------------|
| `krc20FloorPrices` | Get floor prices for KRC20 tokens | `ticker: String?` |
| `tradeStats` | Get trade statistics | `timeFrame: String?, ticker: String?` |
| `soldOrders` | Get recently sold orders | `ticker: String?, minutes: Float?` |
| `lastOrderSold` | Get most recent sold order | - |
| `hotMints` | Get hot minting tokens | `timeInterval: String?` |
| `tokenInfo` | Get comprehensive token info | `ticker: String!` |
| `tokenLogos` | Get token logos | `ticker: String?` |
| `openOrders` | Get tickers with open orders | - |
| `historicalData` | Get historical price/volume data | `timeFrame: String!, ticker: String!` |
| `krc721Mints` | Get recent NFT mints | `ticker: String?` |
| `krc721SoldOrders` | Get sold NFT orders | `ticker: String?, minutes: Float?` |
| `krc721ListedOrders` | Get listed NFT orders | `ticker: String?` |
| `krc721TradeStats` | Get NFT trade statistics | `timeFrame: String?, ticker: String?` |
| `krc721HotMints` | Get hot minting NFT collections | `timeInterval: String?` |
| `krc721FloorPrices` | Get NFT floor prices | `ticker: String?` |
| `krc721CollectionInfo` | Get NFT collection info | `ticker: String!` |
| `nftMetadata` | Get NFT metadata | `ticker: String!, tokenId: Int!` |
| `knsSoldOrders` | Get sold KNS domain orders | `minutes: Float?` |
| `knsTradeStats` | Get KNS trade statistics | `timeFrame: String?, asset: String?` |
| `knsListedOrders` | Get listed KNS domains | - |

For more GraphQL examples and testing guides, see [GRAPHQL_TESTING.md](GRAPHQL_TESTING.md).

---

### REST API Endpoints

#### KRC20 Token Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /v1/api/kaspa/krc20/floor-prices` | Get floor prices for all KRC20 tokens |
| `GET /v1/api/kaspa/krc20/trade-stats` | Get trade statistics |
| `GET /v1/api/kaspa/krc20/sold-orders` | Get recently sold orders |
| `GET /v1/api/kaspa/krc20/hot-mints` | Get hot minting tokens |
| `GET /v1/api/kaspa/krc20/token-info/{ticker}` | Get comprehensive token information |

#### KRC721 NFT Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /v1/api/kaspa/krc721/mints` | Get recent NFT mints |
| `GET /v1/api/kaspa/krc721/sold-orders` | Get sold NFT orders |
| `GET /v1/api/kaspa/krc721/listed-orders` | Get listed NFT orders |
| `GET /v1/api/kaspa/krc721/trade-stats` | Get NFT trade statistics |
| `GET /v1/api/kaspa/krc721/floor-prices` | Get NFT floor prices |
| `GET /v1/api/kaspa/krc721/collection-info/{ticker}` | Get NFT collection information |
| `GET /v1/api/kaspa/krc721/metadata/{ticker}/{tokenId}` | Get NFT metadata |

#### KNS Domain Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /v1/api/kaspa/kns/sold-orders` | Get sold KNS domain orders |
| `GET /v1/api/kaspa/kns/trade-stats` | Get KNS trade statistics |
| `GET /v1/api/kaspa/kns/listed-orders` | Get listed KNS domains |

#### System Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check endpoint |
| `GET /metrics` | Prometheus metrics |
| `GET /v1/api/kaspa/cache/stats` | Cache statistics and hit rates |
| `GET /swagger-ui` | Interactive API documentation |

---

## ⚡ Performance & Caching

### Cache Architecture

The API uses a **tiered caching strategy** for optimal performance:

```
Request → Redis (Hot Cache) → Parquet (Warm/Cold Cache) → Kaspa.com API
   ↓            ↓                      ↓                        ↓
<10ms      <50ms                  <100ms                   500-2000ms
```

**Cache Layers:**

1. **Redis (Hot Cache)** - In-memory, sub-10ms latency
   - Frequently accessed data
   - Short TTL for real-time data
   - Connection pooling for high throughput

2. **Parquet (Warm/Cold Cache)** - Persistent storage, <50ms latency
   - Historical data storage
   - Longer TTL for less volatile data
   - Efficient columnar format

3. **Kaspa.com API** - Last resort, 500-2000ms latency
   - Only fetched on cache miss
   - Rate limited to protect upstream API
   - Automatically populates both cache layers

### Cache Hit Rate Goals

**Target Performance Metrics:**

| Metric | Target | Current |
|--------|--------|---------|
| **Overall Cache Hit Rate** | **>90%** | Tracked |
| **Redis Hit Rate** | **>70%** | Tracked |
| **Parquet Hit Rate** | **>20%** | Tracked |
| **API Response Time (cached)** | **<50ms (p95)** | Tracked |
| **API Response Time (uncached)** | **<2s (p95)** | Tracked |

**Cache TTL Strategy:**

| Data Type | Redis TTL | Parquet TTL | Example |
|-----------|-----------|-------------|---------|
| **Hot Data** | 30 seconds | 5 minutes | Floor prices, recent orders |
| **Warm Data** | 5 minutes | 15 minutes | Trade stats, token stats |
| **Cold Data** | 30 minutes | 1 hour | Token info, historical data |
| **Static Data** | 1 hour | 24 hours | Logos, metadata |

**Monitoring Cache Performance:**

```bash
# Get cache statistics
curl http://localhost:3010/v1/api/kaspa/cache/stats

# Response includes:
# - Total cache hits/misses
# - Per-category statistics
# - Cache size and file counts
# - Hit rates by category
```

**Example Response:**
```json
{
  "total_keys": 1250,
  "total_size_bytes": 52428800,
  "categories_count": 8,
  "cache_hits": 125000,
  "categories": {
    "floor_prices": {
      "keys": 150,
      "size_bytes": 2048000,
      "hits": 45000,
      "misses": 5000,
      "requests": 50000,
      "description": "KRC20 floor prices"
    }
  }
}
```

**Cache Hit Rate Calculation:**
```
Hit Rate = (cache_hits / total_requests) × 100%
```

**Optimization Tips:**
- ✅ Frequently accessed endpoints benefit from Redis caching
- ✅ Historical data is efficiently stored in Parquet format
- ✅ Cache automatically warms on first request
- ✅ Stale data is automatically refreshed on TTL expiry
- ✅ Rate limiting protects upstream API from overload
- ✅ GraphQL queries are cached just like REST endpoints
- ✅ Cache statistics available via `/v1/api/kaspa/cache/stats`

**Achieving Cache Hit Rate Goals:**

To maintain >90% cache hit rate:
1. **Warm up cache** - Make initial requests to populate cache
2. **Monitor cache stats** - Check `/v1/api/kaspa/cache/stats` regularly
3. **Adjust TTLs** - Modify TTL values in `src/application/cache_service.rs` if needed
4. **Use GraphQL** - Request only needed fields to reduce cache size
5. **Leverage Parquet** - Historical data benefits from persistent Parquet storage

**Cache Performance Monitoring:**

```bash
# Get detailed cache statistics
curl http://localhost:3010/v1/api/kaspa/cache/stats | jq

# Monitor cache hit rate over time
watch -n 5 'curl -s http://localhost:3010/v1/api/kaspa/cache/stats | jq ".cache_hits"'
```

**Expected Cache Behavior:**
- **First request**: Cache miss → API call → Cache population
- **Subsequent requests**: Cache hit → <50ms response
- **After TTL expiry**: Cache miss → Refresh from API → Cache update

---

## 🏗️ Self-Hosting

### Prerequisites
- Docker & Docker Compose
- *(Optional)* GitHub Personal Access Token (for GitHub repository access if needed)

### Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/KaspaDev/KaspaDev-KaspaCom-Gateway-Data.git
   cd KaspaDev-KaspaCom-Gateway-Data
   ```

2. **Configure environment** (optional)
   ```bash
   cp .env.sample .env
   # Optionally add GITHUB_TOKEN if you need GitHub repository access
   ```

3. **Start the services**
   ```bash
   docker compose up -d
   ```

4. **Verify**
   ```bash
   curl http://localhost:8080/health
   # {"status":"ok","version":"0.1.0",...}
   ```

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Envoy Proxy (:8080)                 │
│                    (Load Balancer)                      │
└────────────────────────┬────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   ┌────────┐       ┌────────┐       ┌────────┐
   │ API #1 │       │ API #2 │       │ API #3 │
   │ :3010  │       │ :3010  │       │ :3010  │
   └────┬───┘       └────┬───┘       └────┬───┘
        │                │                │
        └────────────────┴────────────────┘
                         │
                    ┌────┴────┐
                    │ Dragonfly│
                    │ (Redis)  │
                    │  :6379   │
                    └──────────┘
```

---

## 🔧 Configuration

Configuration via `config.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 3010
  allowed_origins: "*"

rate_limit:
  requests_per_minute: 1000

allowed_repos:
  - source: github
    owner: KaspaDev
    repo: Kaspa-Exchange-Data
```

Environment variables:
- `GITHUB_TOKEN` - GitHub Personal Access Token (**optional**, only if accessing GitHub repositories)
- `REDIS_URL` - Redis connection URL (default: `redis://dragonfly:6379`)
- `RUST_LOG` - Log level (default: `info`)
- `LOG_FORMAT` - Log format: `text` or `json` (default: `text`)

---

## 📝 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

Contributions welcome! Please read our [Code of Conduct](CODE_OF_CONDUCT.md) first.

---

<p align="center">
  <strong>Built with ❤️ for the Kaspa ecosystem</strong>
</p>

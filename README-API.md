# KaspaCom Data API Gateway

A production-ready REST and GraphQL API gateway that proxies Kaspa.com L1 Marketplace data with added features like **tiered caching**, **load balancing**, **security isolation**, and **comprehensive observability**. 

This gateway provides access to:
- **KRC20 Tokens**: Floor prices, trade stats, sold orders, hot mints, token info
- **KRC721 NFTs**: Mints, orders, trade stats, floor prices, collection info, metadata
- **KNS Domains**: Sold orders, trade stats, listed orders

## Architecture

The system is containerized and composed of three main layers:

1.  **Envoy Proxy** (Port 8080): The secure entry point. Handles rate limiting and load balancing.
2.  **API Cluster** (Internal): 3 Replicas of the Rust API service.
3.  **Caching Layer** (Port 6379): DragonflyDB (Redis-compatible) for hot cache, plus Parquet storage for warm/cold cache.

## Features

- **Secure Access**: Port 3010 is closed. Access is only via Envoy (Port 8080).
- **High Performance**: 
    - **Tiered Caching**: Redis (hot) + Parquet (warm/cold) for optimal performance
    - **Caching**: Responses are cached with intelligent TTLs (`<10ms` Redis latency).
    - **Load Balancing**: Traffic is distributed across 3 API instances.
- **Safety**: Rate limited to protect upstream Kaspa.com API.
- **Observability**: Prometheus metrics, structured logging, request tracing.
- **GraphQL Support**: Flexible queries with automatic caching.

## Installation & Setup

### Prerequisites

- Docker & Docker Compose
- *(Optional)* A GitHub Personal Access Token for GitHub repository access (if needed)

### Configuration

Create a `.env` file in the root directory:

```bash
# Optional: Only needed if accessing GitHub repositories
GITHUB_TOKEN=your_github_token_here

# Redis connection (default if not set)
REDIS_URL=redis://dragonfly:6379

# Logging
RUST_LOG=info
LOG_FORMAT=text
```

### Running the Service

**Production Mode** (Core Services Only):
```bash
./run-prod.sh
```

**Development Mode** (Includes SonarQube):
```bash
./run-dev.sh
```

The API will be available at `http://localhost:8080`.
SonarQube (Dev only) will be at `http://localhost:9000`.

## API Usage

**Base URL**: `http://localhost:8080`

### GraphQL API

**GraphiQL Playground**: `http://localhost:3010/graphql`

Query Kaspa.com marketplace data with flexible queries:

```bash
curl -X POST http://localhost:3010/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { krc20FloorPrices { ticker floorPrice volume } }"
  }'
```

### REST Endpoints

#### KRC20 Token Endpoints

List tokens with floor prices:
```bash
curl http://localhost:8080/v1/api/kaspa/krc20/floor-prices
```

Get trade statistics:
```bash
curl "http://localhost:8080/v1/api/kaspa/krc20/trade-stats?timeFrame=6h&ticker=SLOW"
```

Get sold orders:
```bash
curl "http://localhost:8080/v1/api/kaspa/krc20/sold-orders?ticker=SLOW&minutes=60"
```

#### KRC721 NFT Endpoints

Get NFT mints:
```bash
curl "http://localhost:8080/v1/api/kaspa/krc721/mints?ticker=BITCOIN"
```

Get NFT metadata:
```bash
curl http://localhost:8080/v1/api/kaspa/krc721/metadata/BITCOIN/1
```

#### KNS Domain Endpoints

Get KNS trade statistics:
```bash
curl "http://localhost:8080/v1/api/kaspa/kns/trade-stats?timeFrame=6h"
```

#### System Endpoints

Health check:
```bash
curl http://localhost:8080/health
```

Cache statistics:
```bash
curl http://localhost:3010/v1/api/kaspa/cache/stats
```

Prometheus metrics:
```bash
curl http://localhost:3010/metrics
```

Interactive API documentation:
```
http://localhost:8080/swagger-ui
```

### Advanced Features

#### Caching
Responses are automatically cached with a tiered strategy:
- **Redis (Hot Cache)**: Sub-10ms latency for frequently accessed data
- **Parquet (Warm/Cold Cache)**: Persistent storage for historical data
- **Header**: Check `X-Cache: HIT` or `MISS` in responses
- **TTL**: Varies by data type (30 seconds to 1 hour)

#### Rate Limiting
- **Limit**: Configurable per-minute rate limits (default: 1000 req/min to Kaspa.com API)
- **Response**: `429 Too Many Requests` if exceeded
- **Protection**: Protects upstream Kaspa.com API from overload

#### Observability
- **Metrics**: Prometheus metrics at `/metrics`
- **Logging**: Structured logging with correlation IDs
- **Tracing**: Request tracing for distributed debugging

## Data Sources

This gateway primarily serves data from:
- **Kaspa.com L1 Marketplace API**: KRC20, KRC721, KNS marketplace data
- **GitHub Repositories** (optional): For historical data storage and aggregation

## License
[MIT](LICENSE)

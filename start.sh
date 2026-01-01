#!/bin/bash

# KaspaCom Data API Gateway - Quick Start Script
# Runs the service on port 8088 for local development
# Production uses port 8080 (configured in config.yaml)

set -e

echo "🚀 Starting KaspaCom Data API Gateway (Development Mode)..."
echo ""
echo "📍 Server will be available at:"
echo "   - API: http://localhost:8088"
echo "   - GraphQL Playground: http://localhost:8088/graphql"
echo "   - Swagger UI: http://localhost:8088/swagger-ui"
echo ""
echo "💡 For production (port 8080), run: PORT=8080 cargo run --release"
echo ""

# Set development port (overrides config.yaml)
export PORT=8088

# Run the service
cargo run --release


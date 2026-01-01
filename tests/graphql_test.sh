#!/bin/bash
# GraphQL API Testing Script
# Usage: ./tests/graphql_test.sh [base_url]

BASE_URL="${1:-http://localhost:3010}"
GRAPHQL_ENDPOINT="${BASE_URL}/graphql"

echo "🧪 Testing GraphQL API at ${GRAPHQL_ENDPOINT}"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASSED=0
FAILED=0

# Function to test GraphQL query
test_query() {
    local name="$1"
    local query="$2"
    local expected_field="$3"
    
    echo -n "Testing: ${name}... "
    
    response=$(curl -s -X POST "${GRAPHQL_ENDPOINT}" \
        -H "Content-Type: application/json" \
        -d "{\"query\": \"${query}\"}")
    
    if echo "$response" | grep -q "\"errors\""; then
        echo -e "${RED}FAILED${NC}"
        echo "  Error: $(echo "$response" | jq -r '.errors[0].message' 2>/dev/null || echo 'Unknown error')"
        FAILED=$((FAILED + 1))
        return 1
    fi
    
    if [ -n "$expected_field" ]; then
        if echo "$response" | jq -e ".data.${expected_field}" > /dev/null 2>&1; then
            echo -e "${GREEN}PASSED${NC}"
            PASSED=$((PASSED + 1))
            return 0
        else
            echo -e "${RED}FAILED${NC}"
            echo "  Expected field '${expected_field}' not found in response"
            FAILED=$((FAILED + 1))
            return 1
        fi
    else
        # Just check if we got data back
        if echo "$response" | jq -e ".data" > /dev/null 2>&1; then
            echo -e "${GREEN}PASSED${NC}"
            PASSED=$((PASSED + 1))
            return 0
        else
            echo -e "${RED}FAILED${NC}"
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi
}

# Test 1: Get all floor prices
test_query \
    "Get all KRC20 floor prices" \
    "query { krc20FloorPrices { ticker floorPrice volume } }" \
    "krc20FloorPrices"

# Test 2: Get floor prices for specific ticker
test_query \
    "Get floor prices for SLOW" \
    "query { krc20FloorPrices(ticker: \"SLOW\") { ticker floorPrice } }" \
    "krc20FloorPrices"

# Test 3: Test with variables (using a simple query)
test_query \
    "Query with variables" \
    "query GetPrices(\$ticker: String) { krc20FloorPrices(ticker: \$ticker) { ticker } }" \
    "krc20FloorPrices"

# Test 4: Schema introspection
test_query \
    "Schema introspection" \
    "query { __schema { queryType { name } } }" \
    "__schema"

# Test 5: Invalid query (should return error)
echo -n "Testing: Invalid query (should error)... "
response=$(curl -s -X POST "${GRAPHQL_ENDPOINT}" \
    -H "Content-Type: application/json" \
    -d '{"query": "query { invalidField { data } }"}')

if echo "$response" | grep -q "\"errors\""; then
    echo -e "${GREEN}PASSED${NC} (correctly returned error)"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}FAILED${NC} (should have returned error)"
    FAILED=$((FAILED + 1))
fi

# Test 6: GraphiQL playground (GET request)
echo -n "Testing: GraphiQL playground... "
response=$(curl -s -o /dev/null -w "%{http_code}" "${GRAPHQL_ENDPOINT}")

if [ "$response" = "200" ]; then
    echo -e "${GREEN}PASSED${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}FAILED${NC} (HTTP ${response})"
    FAILED=$((FAILED + 1))
fi

# Summary
echo ""
echo "=========================================="
echo -e "${GREEN}Passed: ${PASSED}${NC}"
echo -e "${RED}Failed: ${FAILED}${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi


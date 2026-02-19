#!/bin/bash
# ============================================================
# test_cube_crud.sh
# Cube CRUD テストスクリプト
# ============================================================
#
# 使用方法:
#   1. サーバーを起動: make run ARGS="rt" &
#   2. JWTを取得してJWT変数を設定
#   3. このスクリプトを実行: bash scripts/test_cube_crud.sh
#
# 注意: 実行前にJWT変数を設定してください
# ============================================================

set -e

BASE_URL="http://localhost:8888/v1"
JWT="${JWT:-}"

if [ -z "$JWT" ]; then
    echo "Error: JWT environment variable is not set."
    echo "Usage: JWT=\"your_jwt_token\" bash scripts/test_cube_crud.sh"
    exit 1
fi

echo "===== Cube CRUD Test ====="

# ============================================================
# 1. Create Cube
# ============================================================
echo ""
echo "[1] Creating Cube..."
CREATE_RESPONSE=$(curl -sS -m 10 -X POST "$BASE_URL/cubes/create" \
    -H "Authorization: Bearer $JWT" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "Test Cube",
        "description": "A test cube for CRUD verification",
        "embedding_provider": "openai",
        "embedding_model": "text-embedding-3-small",
        "embedding_dimension": 1536,
        "embedding_api_key": "sk-test-key"
    }')

echo "Create Response: $CREATE_RESPONSE"

# Extract cube ID using jq (if available) or grep
if command -v jq &> /dev/null; then
    CUBE_ID=$(echo "$CREATE_RESPONSE" | jq -r '.id')
    CUBE_UUID=$(echo "$CREATE_RESPONSE" | jq -r '.uuid')
else
    CUBE_ID=$(echo "$CREATE_RESPONSE" | grep -o '"id":[0-9]*' | head -1 | cut -d: -f2)
    CUBE_UUID=$(echo "$CREATE_RESPONSE" | grep -o '"uuid":"[^"]*"' | head -1 | cut -d'"' -f4)
fi

if [ -z "$CUBE_ID" ] || [ "$CUBE_ID" = "null" ]; then
    echo "ERROR: Failed to create cube"
    exit 1
fi

echo "Created Cube ID: $CUBE_ID, UUID: $CUBE_UUID"

# ============================================================
# 2. Get Cube
# ============================================================
echo ""
echo "[2] Getting Cube (ID: $CUBE_ID)..."
GET_RESPONSE=$(curl -sS -m 10 -X GET "$BASE_URL/cubes/get/$CUBE_ID" \
    -H "Authorization: Bearer $JWT")

echo "Get Response: $GET_RESPONSE"

# ============================================================
# 3. Search Cubes
# ============================================================
echo ""
echo "[3] Searching Cubes..."
SEARCH_RESPONSE=$(curl -sS -m 10 -X POST "$BASE_URL/cubes/search" \
    -H "Authorization: Bearer $JWT" \
    -H "Content-Type: application/json" \
    -d '{"name": "Test"}')

echo "Search Response: $SEARCH_RESPONSE"

# ============================================================
# 4. Delete Cube
# ============================================================
echo ""
echo "[4] Deleting Cube (ID: $CUBE_ID)..."
DELETE_RESPONSE=$(curl -sS -m 10 -X DELETE "$BASE_URL/cubes/delete?cube_id=$CUBE_ID" \
    -H "Authorization: Bearer $JWT")

echo "Delete Response: $DELETE_RESPONSE"

# ============================================================
# 5. Verify Deletion (should return 404)
# ============================================================
echo ""
echo "[5] Verifying Deletion (expecting 404)..."
VERIFY_RESPONSE=$(curl -sS -m 10 -X GET "$BASE_URL/cubes/get/$CUBE_ID" \
    -H "Authorization: Bearer $JWT")

echo "Verify Response: $VERIFY_RESPONSE"

# ============================================================
# 6. Validation Test (missing required fields - expecting 422)
# ============================================================
echo ""
echo "[6] Testing Validation (missing name - expecting error)..."
VALIDATION_RESPONSE=$(curl -sS -m 10 -X POST "$BASE_URL/cubes/create" \
    -H "Authorization: Bearer $JWT" \
    -H "Content-Type: application/json" \
    -d '{
        "description": "Missing name field"
    }')

echo "Validation Response: $VALIDATION_RESPONSE"

echo ""
echo "===== Cube CRUD Test Complete ====="

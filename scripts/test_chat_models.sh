#!/bin/bash
# ChatModel CRUD テストスクリプト
# Usage: bash scripts/test_chat_models.sh
#
# 前提: サーバーが make run ARGS="rt" で起動済み
# curl ルール: -sS -m 10 を必須で使用（AI エージェント環境でのデッドロック防止）

set -e

# JWT Token (USR: apx_id=1, vdr_id=2, usr_id=3)
JWT="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhcHhfaWQiOjEsInZkcl9pZCI6MiwidXNyX2lkIjozLCJlbWFpbCI6Imthd2F0YUBzaHltZS5uZXQiLCJ0eXBlIjowLCJpc19zdGFmZiI6ZmFsc2UsImV4cCI6MjA4MzIyMjE4Nn0._YjqkaRpfwH9za0yoTCkVh9y80GtZ4UAlyf8irOKT20"
BASE_URL="http://localhost:8888/v1"

echo "=========================================="
echo "ChatModel CRUD 正常系テスト"
echo "=========================================="

# 1. CREATE
echo ""
echo "=== 1. CREATE: 新規 ChatModel 作成 ==="
CREATE_RESULT=$(curl -sS -m 10 -X POST "$BASE_URL/chat_models" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test GPT-4",
    "provider": "openai",
    "model": "gpt-4",
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-test-key-123",
    "max_tokens": 2048,
    "temperature": 0.7
  }')
echo "$CREATE_RESULT" | jq .
CREATED_ID=$(echo "$CREATE_RESULT" | jq -r '.id')
echo "Created ID: $CREATED_ID"

if [ "$CREATED_ID" == "null" ] || [ -z "$CREATED_ID" ]; then
  echo "ERROR: Failed to create ChatModel"
  echo "Response: $CREATE_RESULT"
  exit 1
fi

# 2. GET
echo ""
echo "=== 2. GET: 作成した ChatModel を取得 (ID=$CREATED_ID) ==="
curl -sS -m 10 -X GET "$BASE_URL/chat_models/$CREATED_ID" \
  -H "Authorization: Bearer $JWT" | jq .

# 3. SEARCH
echo ""
echo "=== 3. SEARCH: ChatModel を検索 ==="
curl -sS -m 10 -X POST "$BASE_URL/chat_models/search" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test",
    "limit": 10,
    "offset": 0
  }' | jq .

# 4. UPDATE
echo ""
echo "=== 4. UPDATE: ChatModel を更新 (ID=$CREATED_ID) ==="
curl -sS -m 10 -X PATCH "$BASE_URL/chat_models/$CREATED_ID" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Updated Test GPT-4",
    "max_tokens": 4096
  }' | jq .

# Verify update
echo ""
echo "=== 4b. 更新後の ChatModel を取得して確認 ==="
curl -sS -m 10 -X GET "$BASE_URL/chat_models/$CREATED_ID" \
  -H "Authorization: Bearer $JWT" | jq .

# 5. DELETE
echo ""
echo "=== 5. DELETE: ChatModel を削除 (ID=$CREATED_ID) ==="
curl -sS -m 10 -X DELETE "$BASE_URL/chat_models/$CREATED_ID" \
  -H "Authorization: Bearer $JWT" | jq .

# Verify deletion
echo ""
echo "=== 5b. 削除確認 (404 が返却されるべき) ==="
curl -sS -m 10 -X GET "$BASE_URL/chat_models/$CREATED_ID" \
  -H "Authorization: Bearer $JWT" | jq .

echo ""
echo "=========================================="
echo "正常系テスト完了"
echo "=========================================="

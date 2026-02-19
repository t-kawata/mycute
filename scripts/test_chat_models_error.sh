#!/bin/bash
# ChatModel 異常系テストスクリプト
# Usage: bash scripts/test_chat_models_error.sh

set -e

JWT="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhcHhfaWQiOjEsInZkcl9pZCI6MiwidXNyX2lkIjozLCJlbWFpbCI6Imthd2F0YUBzaHltZS5uZXQiLCJ0eXBlIjowLCJpc19zdGFmZiI6ZmFsc2UsImV4cCI6MjA4MzIyMjE4Nn0._YjqkaRpfwH9za0yoTCkVh9y80GtZ4UAlyf8irOKT20"
BASE_URL="http://localhost:8888/v1"

echo "=========================================="
echo "ChatModel 異常系テスト"
echo "=========================================="

# 1. 存在しない ID
echo ""
echo "=== 1. 404: 存在しない ID の取得 ==="
curl -sS -m 10 -X GET "$BASE_URL/chat_models/9999" \
  -H "Authorization: Bearer $JWT" | jq .

# 2. バリデーションエラー (名前が空)
echo ""
echo "=== 2. 400: Name が空 (Create) ==="
curl -sS -m 10 -X POST "$BASE_URL/chat_models" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "",
    "provider": "openai",
    "model": "gpt-4",
    "api_key": "test",
    "max_tokens": 2048,
    "temperature": 0.7
  }' | jq .

# 3. バリデーションエラー (不正な温度)
echo ""
echo "=== 3. 400: Temperature が範囲外 (3.0) ==="
curl -sS -m 10 -X POST "$BASE_URL/chat_models" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Invalid Temp",
    "provider": "openai",
    "model": "gpt-4",
    "api_key": "test",
    "max_tokens": 2048,
    "temperature": 3.0
  }' | jq .

# 4. 権限/パーティショニング (Update) - 存在しない(または他人の)ID
echo ""
echo "=== 4. 404: 他人(または存在しない)ID の更新 ==="
curl -sS -m 10 -X PATCH "$BASE_URL/chat_models/9999" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"name": "No Access"}' | jq .

echo ""
echo "=========================================="
echo "異常系テストスクリプト作成完了"
echo "=========================================="

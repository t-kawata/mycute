#!/bin/bash
# =============================================================================
# Linux AMD64 ビルドスクリプト
# Docker を使用して Apple Silicon Mac から Linux 向けバイナリをビルドします
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DOCKER_IMAGE="mycute-linux-builder"
DOCKER_FILE="$PROJECT_DIR/docker/Dockerfile.linux-build"
OUTPUT_DIR="$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release"
BINARY_NAME="mycute"

echo "========================================================"
echo "Linux AMD64 ビルド (Docker)"
echo "========================================================"

# 1. Docker イメージのビルド（存在しない場合）
echo "[1/4] Docker イメージを確認中..."
if ! docker image inspect "$DOCKER_IMAGE" > /dev/null 2>&1; then
    echo "  -> イメージが存在しません。ビルド中..."
    docker build \
        -t "$DOCKER_IMAGE" \
        -f "$DOCKER_FILE" \
        "$PROJECT_DIR"
else
    echo "  -> イメージが存在します。スキップ。"
    echo "  -> 再ビルドが必要な場合: docker rmi $DOCKER_IMAGE"
fi

# 2. コンテナでビルドを実行
echo "[2/4] Docker コンテナでクロスビルド中..."
echo "  -> (ネイティブ CPU を使用するためエミュレーションより高速です)"

# Cargo のキャッシュ用ボリュームを作成（ビルド高速化）
docker volume create mycute-cargo-cache > /dev/null 2>&1 || true

# コンテナを実行してクロスビルド
docker run --rm \
    -v "$PROJECT_DIR:/app:delegated" \
    -v "mycute-cargo-cache:/cargo:delegated" \
    -e CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    -e CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ \
    -e CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    -w /app \
    "$DOCKER_IMAGE" \
    cargo build --release --target x86_64-unknown-linux-gnu

# 3. ビルド結果をコピー
echo "[3/4] ビルド結果を配置中..."
mkdir -p "$OUTPUT_DIR"

# target/x86_64-unknown-linux-gnu/release からコピー
# 同一パスの場合はスキップ
if [ -f "$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME" ]; then
    if [ "$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME" != "$OUTPUT_DIR/$BINARY_NAME" ]; then
        cp "$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME" "$OUTPUT_DIR/$BINARY_NAME"
        echo "  -> $OUTPUT_DIR/$BINARY_NAME"
    else
        echo "  -> バイナリは既に $OUTPUT_DIR に出力されています。"
    fi
else
    echo "  -> 警告: バイナリが見つかりません: $PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME"
    echo "  -> ビルドログを確認してください。"
    exit 1
fi

# 4. 完了
echo "[4/4] 完了!"
echo "========================================================"
echo "ビルド成功: $OUTPUT_DIR/$BINARY_NAME"
echo ""
echo "バイナリ情報:"
file "$OUTPUT_DIR/$BINARY_NAME"
echo "========================================================"

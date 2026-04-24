#!/bin/bash
VERSION="v1.4.24"
BASE="https://downloads.getmaxim.ai/bifrost/${VERSION}"

# macOS (Apple Silicon / arm64)
curl -L -o bifrost-http "${BASE}/darwin/arm64/bifrost-http"
xattr -c bifrost-http
chmod +x bifrost-http
tar -czf "bifrost-http-darwin-arm64-${VERSION}.tar.gz" bifrost-http
rm bifrost-http

# Linux (amd64)
curl -L -o bifrost-http "${BASE}/linux/amd64/bifrost-http"
chmod +x bifrost-http
tar -czf "bifrost-http-linux-amd64-${VERSION}.tar.gz" bifrost-http
rm bifrost-http

# Windows (amd64)
curl -L -o bifrost-http.exe "${BASE}/windows/amd64/bifrost-http.exe"
tar -czf "bifrost-http-windows-amd64-${VERSION}.tar.gz" bifrost-http.exe
rm bifrost-http.exe
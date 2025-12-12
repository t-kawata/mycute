#!/bin/sh
docker run -p 8092:8000 \
  -v "$(pwd)":/database \
  -e KUZU_FILE=$1 \
  --rm --name kuzu-explorer \
  kuzudb/explorer:latest
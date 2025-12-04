run:
	cd ./src && CGO_ENABLED=1 CGO_LDFLAGS="-L$$(pwd)/pkg/cognee/db/cozodb/lib/darwin_arm64 -framework Security" go run -ldflags='-extldflags "-Wl,-w"' main.go ${ARGS}
build:
	mkdir -p dist
	cd ./src && ../sh/build -o darwin -a arm64
build-linux-amd64:
	mkdir -p dist
	cd ./src && ../sh/build -o linux -a amd64









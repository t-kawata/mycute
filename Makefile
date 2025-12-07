run:
	cd ./src && CGO_ENABLED=1 CGO_LDFLAGS="-framework Security" go run main.go ${ARGS}
build:
	mkdir -p dist
	cd ./src && ../sh/build -o darwin -a arm64
build-linux-amd64:
	mkdir -p dist
	cd ./src && ../sh/build -o linux -a amd64









# Binary name
NAME = mycute
SWIFT_LIB = target/swift/libSpeechHelper.a
SWIFT_FILES = native/swift/SpeechHelper.swift
SWIFT_HEADERS = native/swift/speech_helper.h
WIN_HELPER_DIR = native/cs/SpeechHelper
WIN_HELPER_PROJ = $(WIN_HELPER_DIR)/SpeechHelper.csproj
# Native AOT output path (Static Library)
WIN_TFM = net10.0-windows10.0.26100.0
# Note: Native AOT shared lib (DLL) is in 'publish' subdirectory, but import lib is in 'native'
WIN_LIB_DIR = $(WIN_HELPER_DIR)/bin/Release/$(WIN_TFM)/win-x64/native
WIN_DLL_DIR = $(WIN_HELPER_DIR)/bin/Release/$(WIN_TFM)/win-x64/publish

.PHONY: build build-dev run run-c run-s run-cs clean check mac-helper windows-helper swift-lib download-models cl-dev installer sync-frontend up-mysql down-mysql conn-mysql

tmp:
	git add .
	git commit -m "tmp: $$(date +'%Y-%m-%d %H:%M:%S')"
	git push origin master

push:
	@OLD_VERSION=$$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2); \
	V1=$$(echo $$OLD_VERSION | cut -d. -f1); \
	V2=$$(echo $$OLD_VERSION | cut -d. -f2); \
	V3=$$(echo $$OLD_VERSION | cut -d. -f3); \
	V3=$$((V3 + 1)); \
	if [ $$V3 -gt 99 ]; then V3=0; V2=$$((V2 + 1)); fi; \
	if [ $$V2 -gt 99 ]; then V2=0; V1=$$((V1 + 1)); fi; \
	NEW_VERSION="$$V1.$$V2.$$V3"; \
	echo "Updating version: $$OLD_VERSION -> $$NEW_VERSION"; \
	$(SED_I) 's/^version = ".*"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	$(SED_I) "s/\"version\": \".*\"/\"version\": \"$$NEW_VERSION\"/" sdk-ts/package.json; \
	$(SED_I) "s/\"version\": \".*\"/\"version\": \"$$NEW_VERSION\"/" tauri.conf.json; \
	$(SED_I) "s/const SW_VERSION = '.*';/const SW_VERSION = '$$NEW_VERSION';/" sdk-ts/src/service-worker/mycute_sw.ts; \
	git add .; \
	git commit -m "v$$NEW_VERSION"; \
	git push origin master

pull-force:
	git fetch origin master
	git reset --hard origin/master

# Default target
all: build

# Build Swift helper library (Static)
$(SWIFT_LIB): $(SWIFT_FILES) $(SWIFT_HEADERS)
	@echo "Building Swift helper library (Static)..."
	@mkdir -p target/swift
	swiftc -emit-library -static \
		-target arm64-apple-macos12 \
		-module-name SpeechHelper \
		-o $(SWIFT_LIB) \
		-import-objc-header native/swift/speech_helper.h \
		$(SWIFT_FILES)
	# Note: No codesign needed for static lib

mac-helper: $(SWIFT_LIB)
swift-lib: mac-helper

# Windows Helper Build (Native AOT Static)
windows-helper:
	@echo "[Windows] Building C# SpeechHelper (Native AOT Static)..."
	dotnet publish $(WIN_HELPER_PROJ) -r win-x64 -c Release

# OS Detection and Variables
ifeq ($(OS),Windows_NT)
    # Windows
    # Pre-build dependency: Build the static library first
    BUILD_DEPENDENCIES = windows-helper
    
    # Environment variables for build.rs
    export SPEECH_HELPER_LIB_DIR=$(abspath $(WIN_LIB_DIR))
    
    # Commands
    CHECK_CMD = cargo check
    BUILD_CMD = cargo build --release
    BUILD_DEV_CMD = cargo build
    RUN_CMD = cargo run
    SETTINGS_FILE = ./settings.json
    SED_I = sed -i
    FIND_SRC = node scripts/find-frontend-src.mjs
    TOUCH_CMD = powershell -Command "(Get-Item 'web/dist/spa/index.html').LastWriteTime = Get-Date"
    LIB_SHERPA = sherpa-onnx-c-api.dll
    LIB_ONNX = onnxruntime.dll
    # Clean sync command for Windows
    CLEAN_SYNC = powershell -Command "if (Test-Path ui/dist) { Remove-Item -Path ui/dist/* -Recurse -Force }; Copy-Item -Path web/dist/spa/* -Destination ui/dist -Recurse -Force"
    MKDIR_UI_DIST = powershell -Command "if (!(Test-Path ui/dist)) { New-Item -ItemType Directory ui/dist }"
    RM_DIR_CMD = node -e "const fs=require('fs'); if(process.argv[1]) fs.rmSync(process.argv[1], {recursive: true, force: true});"
else
    # Mac / Unix
    # Pre-build dependency: Build the static library first
    BUILD_DEPENDENCIES = mac-helper
    
    # Environment variables for build.rs (Mac uses target/swift by default)
    export SPEECH_HELPER_LIB_DIR=$(abspath target/swift)
    export MACOSX_DEPLOYMENT_TARGET=13.3
    export CFLAGS=-mmacosx-version-min=13.3
    export CXXFLAGS=-mmacosx-version-min=13.3

    # Commands
    CHECK_CMD = cargo check
    LIB_SHERPA = libsherpa-onnx-c-api.dylib
    LIB_ONNX = libonnxruntime.1.17.1.dylib
    BUILD_CMD = cargo build --release
    BUILD_DEV_CMD = cargo build
    RUN_CMD = cargo run
    SETTINGS_FILE = ./settings.json
    SED_I = sed -i ''
    FIND_SRC = find web/src web/public -type f 2>/dev/null
    TOUCH_CMD = touch web/dist/spa/index.html
    # Clean sync command for Mac/Unix
    CLEAN_SYNC = rm -rf ui/dist/* && cp -r web/dist/spa/* ui/dist/
    MKDIR_UI_DIST = mkdir -p ui/dist
    RM_DIR_CMD = rm -rf
endif

# Check the project for errors
check: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	$(CHECK_CMD)

# Run unit tests (use TEST_ARGS="..." to pass arguments)
test: $(BUILD_DEPENDENCIES)
	cargo test $(TEST_ARGS)

# Run all unit tests
test-all: $(BUILD_DEPENDENCIES)
	cargo test

# Production build (Release)
build: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	$(BUILD_CMD)

# Development build (Debug)
build-dev: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	$(BUILD_DEV_CMD)

install: build
	@echo "Installing $(NAME)..."
	@sudo cp target/release/$(NAME) /usr/local/bin/$(NAME)
	@sudo chmod +x /usr/local/bin/$(NAME)
	@sudo cp target/release/$(NAME) /usr/local/bin/$(NAME)
	@sudo chmod +x /usr/local/bin/$(NAME)
	# Note: SpeechHelper is now statically linked.
	@sudo cp target/release/libsherpa-onnx-c-api.dylib /usr/local/bin/libsherpa-onnx-c-api.dylib
	@sudo cp target/release/libonnxruntime.1.17.1.dylib /usr/local/bin/libonnxruntime.1.17.1.dylib
	@echo "Fixing RPATH for $(NAME)..."
	@sudo install_name_tool -add_rpath @executable_path /usr/local/bin/$(NAME) 2>/dev/null || true
	@sudo codesign -s - --entitlements entitlements.plist -f /usr/local/bin/$(NAME)
	@sudo codesign -s - -f /usr/local/bin/libSpeechHelper.dylib
	@sudo codesign -s - -f /usr/local/bin/libsherpa-onnx-c-api.dylib
	@sudo codesign -s - -f /usr/local/bin/libonnxruntime.1.17.1.dylib

# Run the project in debug mode (use ARGS="..." to pass arguments)
run: build-dev
ifeq ($(OS),Windows_NT)
	@echo "DLL copy is handled by build.rs"
endif
	@echo "Note: This will trigger an OS elevation prompt (Always Elevate)."
	$(RUN_CMD) -- $(ARGS) -s $(SETTINGS_FILE)

# Run specific roles
run-gui: build-dev
	$(RUN_CMD) -- cl -r gui -s $(SETTINGS_FILE)

run-headless: build-dev
	@echo "Running Server mode (Headless) - Requires Sudo..."
	sudo ./target/debug/$(NAME) cl -r headless -s $(SETTINGS_FILE)

run-owner: build-dev
	@if [ -z "$(PASS)" ]; then echo "\033[1;31mError: PASS is required (e.g. make ro PASS=your_passphrase)\033[0m"; exit 1; fi
	@echo "Running Owner Mode (Headless)..."
	sudo ./target/debug/$(NAME) cl -r headless -s $(SETTINGS_FILE) --owner '$(subst ','\'',$(PASS))'

# Legacy aliases / shortcuts
run-g: run-gui
run-h: run-headless
run-am: build-dev
	@echo "Running Auto-Migration (Headless) - Requires Sudo..."
	sudo ./target/debug/$(NAME) am -s $(SETTINGS_FILE)

# Abbreviations for frequent use
rg: run-gui
rh: run-headless
ra: run-am
ro: run-owner

run-web:
	cd web && pnpm quasar dev

# Clean build artifacts
clean:
	cargo clean
	@$(RM_DIR_CMD) target/swift
	@$(RM_DIR_CMD) ui/dist
ifeq ($(OS),Windows_NT)
	cd $(WIN_HELPER_DIR) && dotnet clean
endif

cl-dev: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	cargo tauri dev -- cl

# cl-build: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
# 	cargo tauri build

# ============================================================
# ターゲット: installer (配布用インストーラーの生成)
# ============================================================
#
# 【背景】
#   本アプリケーションは sherpa-onnx / onnxruntime のネイティブ動的ライブラリ
#   (.dylib / .dll) に依存している。これらのファイルは sherpa-rs-sys クレートの
#   build.rs により、ビルド時に target/release/ へ自動コピーされる。
#   しかし、Tauri のインストーラー生成機能 (cargo tauri build) は、
#   これらの動的ライブラリを自動で同封してはくれない。
#   明示的に bundle.resources で指定する必要がある。
#
# 【なぜ tauri.conf.json に直接書かないのか】
#   Tauri のビルドスクリプト (tauri_build::build()) は、cargo check の段階でも
#   resources に指定された全ファイルの物理的な存在を検証する。
#   target/release/ のファイルは release ビルド後にしか存在しないため、
#   tauri.conf.json に常時記載すると make check (cargo check) が必ず失敗する。
#   この問題は tauri.macos.conf.json 等のプラットフォーム別設定でも解消しない
#   （同様にマージ後にチェックが走るため）。
#
# 【解決策: --config による実行時注入 (JSON Merge Patch / RFC 7396)】
#   Tauri v2 の --config フラグは、JSON Merge Patch (RFC 7396) の仕様に従い、
#   指定した JSON オブジェクトを tauri.conf.json に「部分的に上書きマージ」する。
#   つまり、--config で渡したキーだけが上書きされ、
#   それ以外の設定（アイコン、セキュリティ等）は tauri.conf.json の値がそのまま維持される。
#   これにより、リソース設定は「インストーラー生成時にのみ」有効となり、
#   通常の開発サイクル (make check, make run) に一切影響を与えない。
#
# 【macOS での RPATH について】
#   Tauri の bundle.resources で同封されたファイルは、macOS の AppBundle 内では
#   Contents/Resources/ に配置される（実行ファイルは Contents/MacOS/ にある）。
#   この物理的な隔離を橋渡しするため、build.rs において実行バイナリの RPATH に
#   @loader_path/../Resources を追加し、OS のダイナミックローダー (dyld) が
#   Resources フォルダからもライブラリを探索できるようにしている。
#
# 【Windows での配置について】
#   Windows 版の Tauri インストーラー (NSIS) は、bundle.resources で指定した
#   ファイルを実行ファイル (.exe) と同じディレクトリにフラットに配置する。
#   そのため、OS の標準的な DLL 検索順序により自動的に発見・ロードされる。
# ============================================================
# ============================================================
ifeq ($(OS),Windows_NT)
INSTALLER_RESOURCES_CONFIG = {"bundle":{"resources":{"target/release/$(LIB_SHERPA)":"$(LIB_SHERPA)","target/release/$(LIB_ONNX)":"$(LIB_ONNX)","target/release/SpeechHelper.dll":"SpeechHelper.dll"}}}
else
INSTALLER_RESOURCES_CONFIG = {"bundle":{"resources":{"target/release/$(LIB_SHERPA)":"$(LIB_SHERPA)","target/release/$(LIB_ONNX)":"$(LIB_ONNX)"}}}
endif

installer: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	@echo "Ensuring native libraries are in target/release..."
ifeq ($(OS),Windows_NT)
	@powershell -Command "if (!(Test-Path target/release/deps/$(LIB_SHERPA)) -or !(Test-Path target/release/deps/$(LIB_ONNX))) { \
		Write-Host '------------------------------------------------------------' -ForegroundColor Red; \
		Write-Host 'ERROR: Native libraries not found.' -ForegroundColor Red; \
		Write-Host \"$(LIB_SHERPA) or $(LIB_ONNX) does not exist in target/release/deps/.\" -ForegroundColor Yellow; \
		Write-Host 'Please run a release build (make build) first to generate the libraries, then try again.' -ForegroundColor Red; \
		Write-Host '------------------------------------------------------------' -ForegroundColor Red; \
		exit 1; \
	}"
	@powershell -Command "if (!(Test-Path 'target/release/$(LIB_SHERPA)')) { Copy-Item 'target/release/deps/$(LIB_SHERPA)' 'target/release/' }; if (!(Test-Path 'target/release/$(LIB_ONNX)')) { Copy-Item 'target/release/deps/$(LIB_ONNX)' 'target/release/' }"
else
	@if [ ! -f target/release/deps/$(LIB_SHERPA) ] || [ ! -f target/release/deps/$(LIB_ONNX) ]; then \
		echo "------------------------------------------------------------"; \
		echo "\033[0;31mERROR: Native libraries not found.\033[0m"; \
		echo "$(LIB_SHERPA) or $(LIB_ONNX) does not exist in target/release/deps/."; \
		echo "\033[0;31mPlease run a release build (make build) first to generate the libraries, then try again.\033[0m"; \
		echo "------------------------------------------------------------"; \
		exit 1; \
	fi
	@rsync -a target/release/deps/$(LIB_SHERPA) target/release/
	@rsync -a target/release/deps/$(LIB_ONNX) target/release/
endif
	@echo "Building installer with native library resources..."
	cargo tauri build --config '$(INSTALLER_RESOURCES_CONFIG)'
	@echo "Copying installer to dist folder..."
ifeq ($(OS),Windows_NT)
	@node -e "const fs=require('fs');const path=require('path');const v=fs.readFileSync('Cargo.toml','utf8').match(/^version = [^0-9]*([0-9\.]+)/m)[1];const d='dist/win/v'+v;fs.mkdirSync(d,{recursive:true});const src='target/release/bundle/nsis';if(fs.existsSync(src)){fs.readdirSync(src).filter(f=>f.endsWith('.exe')&&f.includes(v)).forEach(f=>fs.copyFileSync(path.join(src,f),path.join(d,f)));console.log('\x1b[32mInstaller successfully copied to '+d+'/\x1b[0m');}else{console.log('\x1b[31mError: Target directory '+src+' not found.\x1b[0m');}"
else
	@APP_VERSION=$$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2); \
	mkdir -p "dist/mac/v$$APP_VERSION"; \
	cp -a target/release/bundle/dmg/*.dmg "dist/mac/v$$APP_VERSION/"; \
	echo "\033[1;32mInstaller successfully copied to dist/mac/v$$APP_VERSION/\033[0m"
endif

# Frontend sources for smart build (exclude large node_modules)
FRONTEND_SRC = $(shell $(FIND_SRC))
FRONTEND_CONFIG = web/quasar.config.ts web/package.json web/pnpm-lock.yaml

# The actual build happens only when sources or config change
web/dist/spa/index.html: $(FRONTEND_SRC) $(FRONTEND_CONFIG)
	@echo "Frontend sources changed. Rebuilding Quasar frontend..."
	cd web && pnpm quasar build
	@$(TOUCH_CMD)

build-frontend: web/dist/spa/index.html

sync-frontend: build-frontend
	@echo "Syncing frontend assets..."
	@$(MKDIR_UI_DIST)
	$(CLEAN_SYNC)



# Directory for models
MODEL_DIR = ./models

# Download model files from Hugging Face / GitHub
download-models:
	@echo "Downloading model files..."
	@mkdir -p $(MODEL_DIR)
	# Download Silero VAD (Required for OpenAI engine)
	@if [ ! -f $(MODEL_DIR)/silero_vad.onnx ]; then \
		echo "Downloading silero_vad.onnx..."; \
		curl -L -o $(MODEL_DIR)/silero_vad.onnx https://huggingface.co/t-kawata/mycute/resolve/main/silero_vad.onnx; \
	fi
	# Download Silero VAD Int8 (Required for OpenAI engine)
	@if [ ! -f $(MODEL_DIR)/silero_vad.int8.onnx ]; then \
		echo "Downloading silero_vad.int8.onnx..."; \
		curl -L -o $(MODEL_DIR)/silero_vad.int8.onnx https://huggingface.co/t-kawata/mycute/resolve/main/silero_vad.int8.onnx; \
	fi
	# Download TEN VAD (Optional but mentioned in settings)
	@if [ ! -f $(MODEL_DIR)/ten_vad.onnx ]; then \
		echo "Downloading ten_vad.onnx..."; \
		curl -L -o $(MODEL_DIR)/ten_vad.onnx https://huggingface.co/t-kawata/mycute/resolve/main/ten_vad.onnx; \
	fi
	# Download TEN VAD Int8 (Optional but mentioned in settings)
	@if [ ! -f $(MODEL_DIR)/ten_vad.int8.onnx ]; then \
		echo "Downloading ten-vad.int8.onnx..."; \
		curl -L -o $(MODEL_DIR)/ten-vad.int8.onnx https://huggingface.co/t-kawata/mycute/resolve/main/ten-vad.int8.onnx; \
	fi
	# Download tokens.txt for speech recognition
	@if [ ! -f $(MODEL_DIR)/tokens.txt ]; then \
		echo "Downloading tokens.txt..."; \
		curl -L -o $(MODEL_DIR)/tokens.txt "https://huggingface.co/t-kawata/mycute/resolve/main/tokens.txt"; \
	fi
	# Download GTCRN speech enhancement model
	@echo "Downloading GTCRN speech enhancement model..."
	@if [ ! -f $(MODEL_DIR)/gtcrn.onnx ]; then \
		echo "Downloading gtcrn.onnx..."; \
		curl -L -o $(MODEL_DIR)/gtcrn.onnx "https://huggingface.co/t-kawata/mycute/resolve/main/gtcrn.onnx"; \
	fi
	@echo "Model download complete!"

# -----------------------------------------------------------------------------

# ============================================================
# ターゲット: up-mysql (MySQL コンテナ起動)
# ============================================================
up-mysql:
	cd ./docker && docker compose up -d mysql

# ============================================================
# ターゲット: down-mysql (MySQL コンテナ停止)
# ============================================================
down-mysql:
	cd ./docker && docker compose stop mysql

# ============================================================
# ターゲット: conn-mysql (MySQL コンテナ接続)
# ============================================================
conn-mysql:
	mysql -h 127.0.0.1 -D mycute -u asterisk -p

# ============================================================
# ターゲット: up-postgres (PostgreSQL コンテナ起動)
# ============================================================
up-postgres:
	cd ./docker && docker compose up -d postgres

# ============================================================
# ターゲット: down-postgres (PostgreSQL コンテナ停止)
# ============================================================
down-postgres:
	cd ./docker && docker compose stop postgres

# ============================================================
# ターゲット: conn-postgres (PostgreSQL コンテナ接続)
# ============================================================
conn-postgres:
	PGPASSWORD=yu51043chie3 psql -h 127.0.0.1 -U asterisk -d mycute

# ============================================================
# ターゲット: conn-sqlite (SQLite データベース接続)
# ============================================================
conn-sqlite:
	@echo "Connecting to SQLite database at disk/db/mycute.sqlite..."
	sqlite3 $(HOME)/.mycute/db/mycute.sqlite

# ============================================================
# ターゲット: cl-build-linux-amd64 (Linux AMD64 ビルド)
# ============================================================
# cl-build-linux-amd64:
# 	@echo "Building mycute for Linux (AMD64) via Docker..."
# 	./scripts/build-linux.sh

# ============================================================
# ターゲット: build-sdk-ts (SDK Build with Version Sync)
# ============================================================
build-sdk-ts:
	@echo "Syncing SDK version with Cargo.toml..."
	@node ./scripts/gen-ts-constants.mjs
	@echo "Building SDK..."
	cd sdk-ts && pnpm install && pnpm run build

# ============================================================
# ターゲット: gen-migration (SeaORM マイグレーションファイル生成)
# ============================================================
gen-migration:
	@if [ -z "${NAME}" ]; then echo "\033[1;31mError: NAME is empty.\033[0m"; exit 1; fi
	mkdir -p ./src/migration && touch ./src/migration/mod.rs
	sea-orm-cli migrate generate ${NAME} -d ./src/migration

# ============================================================
# ターゲット: migrate-refresh (マイグレーションリフレッシュ)
# ============================================================
migrate-refresh: build-dev
	@echo "Dropping all tables and seaql_migrations..."
	@echo "Running fresh migrations..."
	$(RUN_CMD) -- am --refresh -s $(SETTINGS_FILE)

# ============================================================
# ターゲット: migrate-fresh (マイグレーション完全リセット)
# ============================================================
migrate-fresh: build-dev
	@echo "Dropping all tables WITHOUT rollback (Nuclear)..."
	@echo "Running fresh migrations..."
	$(RUN_CMD) -- am --fresh -s $(SETTINGS_FILE)

# ============================================================
# ターゲット: migrate-up (SeaORM マイグレーション実行)
# ============================================================
migrate-up: build-dev
	$(RUN_CMD) -- am -s $(SETTINGS_FILE)

# ============================================================
# ターゲット: gen-entities (SeaORM エンティティファイル生成)
# ============================================================
#
# 使用方法:
#   make gen-entities [DRIVER=<driver>] [HOST=<host>]
#
# 引数:
#   DRIVER: データベースの種類 (sqlite | mysql | postgres)
#     - sqlite: (デフォルト) disk/db/mycute.sqlite を使用。マイグレーション自動実行。
#     - mysql:  MySQLサーバーに接続。要 HOST 指定。
#     - postgres: PostgreSQLサーバーに接続。要 HOST 指定。
#   HOST: データベースサーバーのホスト名 (mysql, postgres 利用時必須)
#
# 例:
#   make gen-entities                       # SQLite (デフォルト)
#   make gen-entities DRIVER=mysql HOST=db  # MySQL
#   make gen-entities DRIVER=postgres HOST=db # PostgreSQL
# ============================================================
gen-entities:
	@echo "Generating entities..."
	@mkdir -p ./src/entities && touch ./src/entities/mod.rs && touch ./src/entities/prelude.rs
	@# Default to sqlite if DRIVER is not set
	@DRIVER=$${DRIVER:-sqlite}; \
	echo "Target Driver: $$DRIVER"; \
	if [ "$$DRIVER" = "sqlite" ]; then \
		echo "Using SQLite..."; \
		mkdir -p $(HOME)/.mycute/db; \
		DB_URL="sqlite://$(HOME)/.mycute/db/mycute.sqlite?mode=rwc"; \
		echo "Ensuring SQLite schema is up-to-date..."; \
	elif [ "$$DRIVER" = "mysql" ]; then \
		if [ -z "${HOST}" ]; then echo "\033[1;31mError: HOST is required for mysql (e.g. make gen-entities DRIVER=mysql HOST=127.0.0.1)\033[0m"; exit 1; fi; \
		DB_URL="mysql://asterisk:yu51043chie3@${HOST}:3306/mycute"; \
		echo "Using MySQL (HOST=${HOST})..."; \
	elif [ "$$DRIVER" = "postgres" ]; then \
		if [ -z "${HOST}" ]; then echo "\033[1;31mError: HOST is required for postgres (e.g. make gen-entities DRIVER=postgres HOST=127.0.0.1)\033[0m"; exit 1; fi; \
		DB_URL="postgres://asterisk:yu51043chie3@${HOST}:5432/mycute"; \
		echo "Using PostgreSQL (HOST=${HOST})..."; \
	else \
		echo "\033[1;31mError: Unknown driver '$$DRIVER'. Supported: sqlite, mysql, postgres\033[0m"; \
		exit 1; \
	fi; \
	if [ -n "${DB_URL_OVERRIDE}" ]; then \
		DB_URL="${DB_URL_OVERRIDE}"; \
		echo "Using overridden DB_URL..."; \
	fi; \
	echo "Generating entities from $$DB_URL ..."; \
	sea-orm-cli generate entity \
		--with-serde both \
		--lib \
		-u "$$DB_URL" \
		-o ./src/entities
	@echo "Patching entity files for UTC timestamp behavior..."
	@for file in ./src/entities/*.rs; do \
		basename=$$(basename "$$file"); \
		if [ "$$basename" != "mod.rs" ] && [ "$$basename" != "prelude.rs" ]; then \
			if ! grep -q "impl_utc_timestamp_behavior" "$$file"; then \
				sed -i '' 's/impl ActiveModelBehavior for ActiveModel {}/\/\/ impl ActiveModelBehavior for ActiveModel {}\ncrate::impl_utc_timestamp_behavior!(ActiveModel);/' "$$file"; \
				sed -i '' 's/DateTimeUtc/DateTime/g' "$$file"; \
				echo "  Patched: $$basename"; \
			else \
				echo "  Skipped (already patched): $$basename"; \
			fi \
		fi \
	done
	@echo "\033[1;32mEntity generation complete.\033[0m"

# ============================================================
# ターゲット: build-15-owner-passphrase (オーナー鍵生成)
# ============================================================
build-15-owner-passphrase:
	$(RUN_CMD) -- og --file ./passphrases.txt
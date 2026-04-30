# Binary name (Default if not in .env)
NAME = mycute
-include .env
export

# Edition-aware variables
APP_SERVER_NAME ?= mycute-server
APP_DISPLAY_NAME ?= MYCUTE

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

.PHONY: build build-dev run clean check mac-helper windows-helper swift-lib download-models cl-dev installer sync-frontend up-mysql down-mysql conn-mysql rg server-dev clean-logs rh release release-login all all-release all-mycute all-mycute-release all-necoasovi all-necoasovi-release setup-edition

tmp:
	git add .
	git commit -m "tmp: $$(date +'%Y-%m-%d %H:%M:%S')"
	git push origin master

push:
	@OLD_VERSION=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	V1=$$(echo $$OLD_VERSION | cut -d. -f1); \
	V2=$$(echo $$OLD_VERSION | cut -d. -f2); \
	V3=$$(echo $$OLD_VERSION | cut -d. -f3); \
	V3=$$((V3 + 1)); \
	if [ $$V3 -gt 99 ]; then V3=0; V2=$$((V2 + 1)); fi; \
	if [ $$V2 -gt 99 ]; then V2=0; V1=$$((V1 + 1)); fi; \
	NEW_VERSION="$$V1.$$V2.$$V3"; \
	echo "Updating version: $$OLD_VERSION -> $$NEW_VERSION"; \
	$(SED_I) 's/MYCUTE_VERSION: &str = "v.*"/MYCUTE_VERSION: \&str = "v'$$NEW_VERSION'"/' src/constants.rs; \
	$(SED_I) "s/\"version\": \".*\"/\"version\": \"$$NEW_VERSION\"/" sdk-ts/package.json; \
	$(SED_I) "s/\"version\": \".*\"/\"version\": \"$$NEW_VERSION\"/" tauri.conf.json; \
	$(SED_I) "s/const SW_VERSION = '.*';/const SW_VERSION = '$$NEW_VERSION';/" sdk-ts/src/service-worker/mycute_sw.ts; \
	make build-sdk-ts; \
	git add .; \
	git commit -m "v$$NEW_VERSION"; \
	git push origin master; \
	cd ../mycute-pub && make push

pull:
	git fetch origin master
	git reset --hard origin/master

# Default target: GUI Installer + Server Binary
# バージョン記録・ロック（last_version.txt）は make all でのみ行われる。
# make installer / make server 単体ではバージョンチェックをスキップする。
# ============================================================
# ターゲット: all (全エディション一括デバッグビルド)
# すべてのエディションを順番にビルドする。
# ============================================================
all:
	@$(MAKE) check-version
	@$(MAKE) all-mycute SKIP_VERSION_CHECK=1
	@$(MAKE) all-necoasovi SKIP_VERSION_CHECK=1
	@$(MAKE) record-version
	@echo "\033[1;32mall: All editions built successfully.\033[0m"

# ============================================================
# ターゲット: all-release (全エディション一括リリースビルド)
# すべてのエディションを順番にリリースビルドし、各リポジトリにリリースする。
# ============================================================
all-release:
	@$(MAKE) check-version
	@$(MAKE) all-mycute-release SKIP_VERSION_CHECK=1
	@$(MAKE) all-necoasovi-release SKIP_VERSION_CHECK=1
	@$(MAKE) record-version
	@echo "\033[1;32mall-release: All editions released successfully.\033[0m"

# ============================================================
# ターゲット: all-mycute (mycute エディション デバッグビルド)
# ============================================================
all-mycute:
	@echo "\033[1;34m[Edition: mycute] Applying edition settings...\033[0m"
	@node scripts/apply-edition.js mycute
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) check-version; fi
	@$(MAKE) generate-icons
	@$(MAKE) server installer
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) record-version; fi
	@echo "\033[1;32m[Edition: mycute] Build complete.\033[0m"

# ============================================================
# ターゲット: all-mycute-release (mycute エディション リリースビルド + GitHub リリース)
# ============================================================
all-mycute-release:
	@echo "\033[1;34m[Edition: mycute] Applying edition settings and building release...\033[0m"
	@node scripts/apply-edition.js mycute
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) check-version; fi
	@$(MAKE) generate-icons
	@$(MAKE) server installer
	@echo "\033[1;34m[Edition: mycute] Releasing to GitHub...\033[0m"
	@. ./.env && V=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+') && \
	if [ "$(OS)" = "Windows_NT" ]; then $(MAKE) release dist/win/v$$V; else $(MAKE) release dist/mac/v$$V; fi
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) record-version; fi
	@echo "\033[1;32m[Edition: mycute] Release complete.\033[0m"

# ============================================================
# ターゲット: all-necoasovi (neco-asovi エディション デバッグビルド)
# ============================================================
all-necoasovi:
	@echo "\033[1;34m[Edition: neco-asovi] Applying edition settings...\033[0m"
	@node scripts/apply-edition.js neco-asovi
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) check-version; fi
	@$(MAKE) generate-icons
	@$(MAKE) server installer
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) record-version; fi
	@echo "\033[1;32m[Edition: neco-asovi] Build complete.\033[0m"

# ============================================================
# ターゲット: all-necoasovi-release (neco-asovi エディション リリースビルド + GitHub リリース)
# ============================================================
all-necoasovi-release:
	@echo "\033[1;34m[Edition: neco-asovi] Applying edition settings and building release...\033[0m"
	@node scripts/apply-edition.js neco-asovi
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) check-version; fi
	@$(MAKE) generate-icons
	@$(MAKE) server installer
	@echo "\033[1;34m[Edition: neco-asovi] Releasing to GitHub...\033[0m"
	@. ./.env && V=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+') && \
	if [ "$(OS)" = "Windows_NT" ]; then $(MAKE) release dist/win/v$$V; else $(MAKE) release dist/mac/v$$V; fi
	@if [ "$(SKIP_VERSION_CHECK)" != "1" ]; then $(MAKE) record-version; fi
	@echo "\033[1;32m[Edition: neco-asovi] Release complete.\033[0m"
# 单体で各設定ファイルを書き換えるだけ。ビルドは行わない。
# 使用例: make setup-edition EDITION=neco-asovi
# ============================================================
setup-edition:
	@if [ -z "$(EDITION)" ]; then echo "\033[1;31mError: EDITION is required (e.g. make setup-edition EDITION=neco-asovi)\033[0m"; exit 1; fi
	@node scripts/apply-edition.js $(EDITION)
	@$(MAKE) generate-icons
	@echo "\033[1;32mEdition '$(EDITION)' applied. Run 'source .env' to load environment variables.\033[0m"

# ============================================================
# ターゲット: generate-icons (アイコンの動的生成)
# .env から読み込んだ APP_ICON_PATH を元に Frontend/Native アイコンを生成。
# ============================================================
generate-icons:
	@. ./.env && \
	if [ -z "$$APP_ICON_PATH" ]; then echo "\033[1;31mError: APP_ICON_PATH is not set in .env\033[0m"; exit 1; fi && \
	if [ ! -f "$$APP_ICON_PATH" ]; then echo "\033[1;31mError: Icon file not found at $$APP_ICON_PATH\033[0m"; exit 1; fi && \
	echo "Generating icons from $$APP_ICON_PATH..." && \
	(cd web && npx icongenie generate -i ../$$APP_ICON_PATH --quality 12 || { echo "\033[1;31mIcongenie failed\033[0m"; exit 1; }) && \
	(cargo tauri icon $$APP_ICON_PATH || { echo "\033[1;31mTauri icon generation failed\033[0m"; exit 1; })

# ============================================================
# ターゲット: check-version (バージョンの重複ビルド防止チェック)
# ============================================================
# make all の先頭でのみ呼び出され、全成果物で共通のバージョン重複チェックを行う。
# 合格すれば installer と server の両ビルドを実行する。
# ============================================================
check-version:
	@echo "Checking version before full build..."
ifeq ($(OS),Windows_NT)
	@node -e "const fs=require('fs'); \
		const curr=fs.readFileSync('src/constants.rs','utf8').match(/MYCUTE_VERSION[^0-9]*([0-9\.]+)/)[1]; \
		const path='dist/last_build_version.txt'; \
		if(fs.existsSync(path)){ \
			const last=fs.readFileSync(path,'utf8').trim(); \
			if(curr===last){ \
				console.log('\\x1b[31mError: Version ' + curr + ' has already been built. Please run \\x22make push\\x22 to increment the version first.\\x1b[0m'); \
				process.exit(1); \
			} \
		}"
else
	@CURR_VERSION=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	LAST_VER_FILE="dist/last_build_version.txt"; \
	if [ -f "$$LAST_VER_FILE" ]; then \
		LAST_VERSION=$$(cat "$$LAST_VER_FILE" | tr -d '[:space:]'); \
		if [ "$$CURR_VERSION" = "$$LAST_VERSION" ]; then \
			echo "\033[0;31mError: Version $$CURR_VERSION has already been built. Please run \"make push\" to increment the version first.\033[0m"; \
			exit 1; \
		fi; \
	fi
endif

# ============================================================
# ターゲット: record-version (ビルド成功後のバージョン記録)
# ============================================================
# make all の最後に呼び出され、全成果物の生成成功を記録する。
# ============================================================
record-version:
ifeq ($(OS),Windows_NT)
	@node -e "const fs=require('fs'); \
		const v=fs.readFileSync('src/constants.rs','utf8').match(/MYCUTE_VERSION[^0-9]*([0-9\.]+)/)[1]; \
		if(!fs.existsSync('dist')) fs.mkdirSync('dist',{recursive:true}); \
		fs.writeFileSync('dist/last_build_version.txt', v); \
		console.log('\\x1b[32mVersion ' + v + ' recorded.\\x1b[0m');"
else
	@APP_VERSION=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	mkdir -p dist; \
	echo "$$APP_VERSION" > dist/last_build_version.txt; \
	echo "\033[1;32mVersion $$APP_VERSION recorded in dist/last_build_version.txt\033[0m"
endif

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
    SED_I = sed -i
    FIND_SRC = node scripts/find-frontend-src.mjs
    TOUCH_CMD = powershell -Command "(Get-Item 'web/dist/spa/index.html').LastWriteTime = Get-Date"
    LIB_SHERPA = sherpa-onnx-c-api.dll
    LIB_ONNX = onnxruntime.dll
    # Clean sync command for Windows
    CLEAN_SYNC = powershell -Command "if (!(Test-Path ui/dist)) { New-Item -ItemType Directory ui/dist }; robocopy web\\dist\\spa ui\\dist /MIR /DCOPY:T /NJH /NJS /NDL /NC /NS /NP ; if ((Get-Variable LASTEXITCODE -ValueOnly) -lt 8) { exit 0 } else { exit (Get-Variable LASTEXITCODE -ValueOnly) }"
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
    SED_I = sed -i ''
    FIND_SRC = find web/src web/public -type f 2>/dev/null
    TOUCH_CMD = touch web/dist/spa/index.html
    # Clean sync command for Mac/Unix
    CLEAN_SYNC = rsync -a --delete web/dist/spa/ ui/dist/
    MKDIR_UI_DIST = mkdir -p ui/dist
    RM_DIR_CMD = rm -rf
endif

# Frontend check (Quasar Build + TS SDK)
check-fe: sync-frontend build-sdk-ts
	@echo "Frontend check/build complete."

# Backend check (Rust + Native Dependencies)
check-be: $(BUILD_DEPENDENCIES)
	$(CHECK_CMD)

# Check all (Frontend + Backend)
check-all: check-fe check-be

# ============================================================
# ビルド系コマンド
# ============================================================

# リリースビルド (インストーラー生成用)
build: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	$(BUILD_CMD)
	@echo "Release build complete."

# デバッグビルド (開発用)
build-dev: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	$(BUILD_DEV_CMD)
	@echo "Debug build complete."

# Run unit tests (use TEST_ARGS="..." to pass arguments)
test: $(BUILD_DEPENDENCIES)
	cargo test $(TEST_ARGS)

# Run all unit tests
test-all: $(BUILD_DEPENDENCIES)
	cargo test

# ============================================================
# 実行系コマンド (開発時のショートカット)
# ============================================================

# GUIモードで起動 (Tauriサーバーを利用するためフロントエンドの同期・待機は不要)
rg: clean-logs $(BUILD_DEPENDENCIES) server-dev
	cargo tauri dev --release -- cl

# サーバーモード(デバッグ)でのビルド
# launcher.rs が target/release のバイナリを include_bytes! するため、--release が必須
server-dev:
	cargo build --bin mycute-server-core
	cargo build --release --bin mycute-server-core
	@# core のビルド後に launcher を touch することで、確実に最新の core を埋め込ませる
	$(TOUCH_CMD)
	touch src/launcher.rs
	cargo build --bin mycute-server
	cargo build --release --bin mycute-server

# ログとロックファイルのクリーンアップ
clean-logs:
	rm -f /tmp/$(APP_SERVER_NAME).log
	rm -f $(HOME)/$(APP_DATA_DIR)/$(APP_SERVER_NAME).lock
	rm -f $(HOME)/$(APP_DATA_DIR)/$(APP_SERVER_NAME)-app.lock
	rm -f target/release/.__$(APP_SERVER_NAME)-core
	rm -f target/debug/.__$(APP_SERVER_NAME)-core

# サーバーモード(ヘッドレス)で起動 (要Sudo)
rh: $(BUILD_DEPENDENCIES)
	$(BUILD_DEV_CMD)
	@echo "Running Server mode (Headless) - Requires Sudo..."
	sudo ./target/debug/$(NAME) cl -r headless

# マイグレーション(データベース定義の自動反映)を実行 (要Sudo)
ra: $(BUILD_DEPENDENCIES)
	$(BUILD_DEV_CMD)
	@echo "Running Auto-Migration (Headless) - Requires Sudo..."
	sudo ./target/debug/$(NAME) am

# オーナーモード(初期設定・特権モード)で起動 (PASS=... が必須)
ro: $(BUILD_DEPENDENCIES)
	$(BUILD_DEV_CMD)
	@if [ -z "$(PASS)" ]; then echo "\033[1;31mError: PASS is required (e.g. make ro PASS=your_passphrase)\033[0m"; exit 1; fi
	@echo "Running Owner Mode (Headless)..."
	sudo ./target/debug/$(NAME) cl -r headless --owner '$(subst ','\'',$(PASS))'

# Webブラウザでのフロントエンド単体確認用
run-web:
	cd web && pnpm quasar dev

# ============================================================
# クリーンアップ
# ============================================================
clean:
	cargo clean
	@$(RM_DIR_CMD) target/swift
	@$(RM_DIR_CMD) ui/dist
ifeq ($(OS),Windows_NT)
	cd $(WIN_HELPER_DIR) && dotnet clean
endif

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
#   Contents/Frameworks/ に配置される（実行ファイルは Contents/MacOS/ にある）。
#   この物理的な隔離を橋渡しするため、build.rs において実行バイナリの RPATH に
#   @loader_path/../Frameworks を追加し、OS のダイナミックローダー (dyld) が
#   Frameworks フォルダからもライブラリを探索できるようにしている。
#
# 【Windows での配置について】
#   Windows 版の Tauri インストーラー (NSIS) は、bundle.resources で指定した
#   ファイルを実行ファイル (.exe) と同じディレクトリにフラットに配置する。
#   そのため、OS の標準的な DLL 検索順序により自動的に発見・ロードされる。
# ============================================================
# ============================================================
ifeq ($(OS),Windows_NT)
INSTALLER_RESOURCES_CONFIG = {"bundle":{"resources":{"target/release/$(LIB_SHERPA)":"$(LIB_SHERPA)","target/release/$(LIB_ONNX)":"$(LIB_ONNX)","target/release/SpeechHelper.dll":"SpeechHelper.dll","target/release/vcruntime140.dll":"vcruntime140.dll","target/release/vcruntime140_1.dll":"vcruntime140_1.dll","target/release/msvcp140.dll":"msvcp140.dll"}}}
else
INSTALLER_RESOURCES_CONFIG = {"bundle":{"macOS":{"frameworks":["target/release/$(LIB_SHERPA)","target/release/$(LIB_ONNX)"]}}}
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
	@echo "Copying VC++ Runtime DLLs to target/release/ for bundling..."
	@powershell -Command "Copy-Item 'C:\Windows\System32\vcruntime140.dll' 'target/release/' -ErrorAction SilentlyContinue; Copy-Item 'C:\Windows\System32\vcruntime140_1.dll' 'target/release/' -ErrorAction SilentlyContinue; Copy-Item 'C:\Windows\System32\msvcp140.dll' 'target/release/' -ErrorAction SilentlyContinue"
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
	@node -e "const fs=require('fs');const path=require('path');const v=fs.readFileSync('src/constants.rs','utf8').match(/MYCUTE_VERSION[^0-9]*([0-9\.]+)/)[1];const d='dist/win/v'+v;fs.mkdirSync(d,{recursive:true});const src='target/release/bundle/nsis';if(fs.existsSync(src)){fs.readdirSync(src).filter(f=>f.endsWith('.exe')&&f.includes(v)).forEach(f=>fs.copyFileSync(path.join(src,f),path.join(d,'win-'+f)));fs.writeFileSync('dist/win/last_version.txt', v);console.log('\x1b[32mInstaller successfully copied to '+d+'/\x1b[0m');}else{console.log('\x1b[31mError: Target directory '+src+' not found.\x1b[0m');}"
else
	@APP_VERSION=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	mkdir -p "dist/mac/v$$APP_VERSION"; \
	for f in target/release/bundle/dmg/*.dmg; do \
		[ -e "$$f" ] || continue; \
		cp -a "$$f" "dist/mac/v$$APP_VERSION/mac-$$(basename "$$f")"; \
	done; \
	ditto scripts/macos-setup.command "dist/mac/v$$APP_VERSION/macos-setup.command"; \
	echo "$$APP_VERSION" > dist/mac/last_version.txt; \
	echo "\033[1;32mInstaller and setup script successfully copied to dist/mac/v$$APP_VERSION/\033[0m"
endif

# ============================================================
# ターゲット: server (サーバーバイナリのビルドと dist/ への配置)
# ============================================================
# GUI非依存のスタンドアロンサーバーバイナリ（mycute-server）をビルドし、
# make installer と同じ dist/mac(win)/v<バージョン>/ に配置する。
# ファイル名には mac-server- / win-server- プレフィックスとバージョンを含める。
# ============================================================
server: $(BUILD_DEPENDENCIES) sync-frontend build-sdk-ts
	@echo "Ensuring native libraries for server binary packing..."
ifeq ($(OS),Windows_NT)
	@powershell -Command "if (!(Test-Path target/release/$(LIB_SHERPA)) -or !(Test-Path target/release/$(LIB_ONNX)) -or !(Test-Path target/release/SpeechHelper.dll)) { \
		Write-Host '------------------------------------------------------------' -ForegroundColor Red; \
		Write-Host 'ERROR: Native libraries not found in target/release/.' -ForegroundColor Red; \
		Write-Host 'Please run a release build (make build) first to generate the libraries, then try again.' -ForegroundColor Red; \
		Write-Host '------------------------------------------------------------' -ForegroundColor Red; \
		exit 1; \
	}"
else
	@if [ ! -f target/release/$(LIB_SHERPA) ] || [ ! -f target/release/$(LIB_ONNX) ]; then \
		echo "------------------------------------------------------------"; \
		echo "\033[0;31mERROR: Native libraries not found in target/release/.\033[0m"; \
		echo "Please run a release build (make build) first to generate the libraries, then try again."; \
		echo "------------------------------------------------------------"; \
		exit 1; \
	fi
endif
	@echo "Building core server binary (mycute-server-core)..."
	cargo build --release --bin mycute-server-core
	@echo "Building launcher binary (mycute-server) with core and libs packed..."
	cargo build --release --bin mycute-server
	@echo "Copying launcher binary to dist folder..."
ifeq ($(OS),Windows_NT)
	@V=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	RAW_ARCH="$$PROCESSOR_ARCHITECTURE"; \
	if [ "$$RAW_ARCH" = "AMD64" ]; then ARCH="x64"; else ARCH="$$RAW_ARCH"; fi; \
	mkdir -p "dist/win/v$${V}"; \
	cp target/release/mycute-server.exe "dist/win/v$${V}/win-$(APP_SERVER_NAME)_$${V}_$${ARCH}.exe"; \
	echo "Launcher binary copied to dist/win/v$${V}/win-$(APP_SERVER_NAME)_$${V}_$${ARCH}.exe"
else
	@V=$$(grep 'MYCUTE_VERSION' src/constants.rs | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	RAW_ARCH=$$(uname -m); \
	if [ "$$RAW_ARCH" = "arm64" ]; then ARCH="aarch64"; elif [ "$$RAW_ARCH" = "x64" ]; then ARCH="x64"; else ARCH="$$RAW_ARCH"; fi; \
	mkdir -p "dist/mac/v$${V}"; \
	cp target/release/mycute-server "dist/mac/v$${V}/mac-$(APP_SERVER_NAME)_$${V}_$${ARCH}"; \
	echo "\033[1;32mLauncher binary copied to dist/mac/v$${V}/mac-$(APP_SERVER_NAME)_$${V}_$${ARCH}\033[0m"
endif


# ============================================================
# ターゲット: release (GitHub リリースの作成とアップロード)
# ============================================================
# 使用方法: make release <directory>
# 例: make release dist/mac/v1.2.3
# ============================================================
release-login:
	gh auth login

release:
	@if [ -z "$(filter-out $@,$(MAKECMDGOALS))" ]; then \
		echo "\033[1;31mError: Directory is required. (e.g. make release dist/mac/v1.2.3)\033[0m"; \
		exit 1; \
	fi
	@# .env が存在する場合は読み込んで APP_REPO を伝達する。存在しない場合は release.sh 内のデフォルト値を使用する。
	@if [ -f .env ]; then . ./.env; fi; bash scripts/release.sh $(filter-out $@,$(MAKECMDGOALS))


# make release <dir> において、<dir> をターゲットとして扱わないためのダミー
%:
	@:

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
	@echo "Syncing SDK version with src/constants.rs..."
	@node ./scripts/gen-ts-constants.mjs
	@echo "Checking SDK dependencies..."
	@if [ ! -d sdk-ts/node_modules ]; then \
		echo "Installing SDK dependencies..."; \
		cd sdk-ts && pnpm install; \
	fi
	@echo "Building SDK..."
	cd sdk-ts && pnpm run build

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
migrate-refresh: $(BUILD_DEPENDENCIES)
	@echo "Dropping all tables and seaql_migrations..."
	@echo "Running fresh migrations..."
	$(RUN_CMD) -- am --refresh

# ============================================================
# ターゲット: migrate-fresh (マイグレーション完全リセット)
# ============================================================
migrate-fresh: $(BUILD_DEPENDENCIES)
	@echo "Dropping all tables WITHOUT rollback (Nuclear)..."
	@echo "Running fresh migrations..."
	$(RUN_CMD) -- am --fresh

# ============================================================
# ターゲット: migrate-up (SeaORM マイグレーション実行)
# ============================================================
migrate-up: $(BUILD_DEPENDENCIES)
	$(RUN_CMD) -- am

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
		mkdir -p $(HOME)/$(APP_DATA_DIR)/db; \
		DB_URL="sqlite://$(HOME)/$(APP_DATA_DIR)/db/$(APP_SLUG).sqlite?mode=rwc"; \
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
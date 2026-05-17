@echo off
setlocal

:: ============================================================================
:: MYCUTE Cleanup Batch
:: アプリ終了後に独立して MYCUTE_HOME ディレクトリを削除します。
:: このファイルはテンプレートであり、Rust 側から変数置換されて出力されます。
:: ============================================================================

set "TARGET_DIR=__TARGET_DIR__"
set "LOG_FILE=__LOG_FILE__"
set "LOCK_FILE=__LOCK_FILE__"
set "MAX_RETRIES=__MAX_RETRIES__"
set "RETRY_DELAY_SEC=__RETRY_DELAY_SEC__"

:: ログ開始
echo [%DATE% %TIME%] MYCUTE cleanup started > "%LOG_FILE%"
echo [%DATE% %TIME%] Target: %TARGET_DIR% >> "%LOG_FILE%"

:: ロックファイルチェック（多重起動防止）
if exist "%LOCK_FILE%" (
    echo [%DATE% %TIME%] Another cleanup instance is already running. Exiting. >> "%LOG_FILE%"
    del "%~f0"
    exit /b 0
)

:: ロックファイル作成
echo %DATE% %TIME% > "%LOCK_FILE%"

set RETRY_COUNT=0

:LOOP
if not exist "%TARGET_DIR%" goto SUCCESS
echo [%DATE% %TIME%] Attempt %RETRY_COUNT% / %MAX_RETRIES% >> "%LOG_FILE%"

rmdir /s /q "%TARGET_DIR%" 2>> "%LOG_FILE%"
if not exist "%TARGET_DIR%" goto SUCCESS

set /a RETRY_COUNT+=1
if %RETRY_COUNT% geq %MAX_RETRIES% goto FAILED

timeout /t %RETRY_DELAY_SEC% /nobreak >nul
goto LOOP

:SUCCESS
echo [%DATE% %TIME%] SUCCESS: Directory deleted >> "%LOG_FILE%"
del "%LOCK_FILE%" 2>nul
del "%~f0"
exit /b 0

:FAILED
echo [%DATE% %TIME%] FAILED: Directory still exists after %MAX_RETRIES% retries >> "%LOG_FILE%"
del "%LOCK_FILE%" 2>nul
exit /b 1

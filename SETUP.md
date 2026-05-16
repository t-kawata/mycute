# ビルド環境セットアップ

## macOS

### 前提条件

以下はインストール済みであることを前提とする：

- Xcode（Command Line Tools を含む）
- nodenv
- rustup

### 手順

1. フロントエンドの依存関係をインストールする。

```bash
cd web && pnpm install && cd ..
cd slide && pnpm install && cd ..
cd sdk-ts && pnpm install && cd ..
```

2. システム依存パッケージをインストールする。

```bash
brew install cmake
brew install gh
```

3. Claude Code CLI をインストールする。

```bash
curl -fsSL https://claude.ai/install.sh | bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

4. ecc-mycute プラグインをインストールする。

```bash
make install-ecc-mycute
```

5. GitHub CLI でログインする。接続方式は SSH を選ぶこと。

```bash
make release-login
```

6. 最初のビルドを実行する。これは必ず失敗するが、後続の手順で必要なアセットを生成するために必要。

```bash
make build
```

以下のエラーが出れば正常（CORE_BIN_BYTES が参照するバイナリがまだ存在しないため）：

```
error: couldn't read `src/../target/release/mycute-server-core`: No such file or directory (os error 2)
  --> src/launcher.rs:22:32
   |
22 | static CORE_BIN_BYTES: &[u8] = include_bytes!("../target/release/mycute-server-core");
   |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

7. サーバーバイナリを単体ビルドし、上記で不足していたバイナリを生成する。

```bash
make server
```

8. 改めて完全ビルドを実行する。これが成功すればセットアップ完了。

```bash
make build
```

### 確認

`make build` が正常終了すれば、macOS でのビルド環境は整っている。

## Windows

### 前提条件

以下はインストール済みであることを前提とする：

- nodenv
- rustup
- [Build Tools for Visual Studio 2026](https://visualstudio.microsoft.com/ja/downloads/) — 「C++ CMake tools for Windows」を含むワークロードを選択すること（cmake はこれに同梱されるため別途インストール不要）
- [.NET SDK 10.0](https://dotnet.microsoft.com/en-us/download/dotnet/10.0)

### 手順

1. フロントエンドの依存関係をインストールする。

```bash
cd web && pnpm install && cd ..
cd slide && pnpm install && cd ..
cd sdk-ts && pnpm install && cd ..
```

2. GitHub CLI をインストールする（管理者権限の PowerShell で実行すること）。

```powershell
choco install gh
```

3. Claude Code CLI をインストールする（管理者権限の PowerShell で実行すること）。

```powershell
irm https://claude.ai/install.ps1 | iex
```

PowerShell を再起動するとパスが反映される。あるいは CMD でもインストール可能：

```cmd
curl -fsSL https://claude.ai/install.cmd -o install.cmd && install.cmd && del install.cmd
```

4. ecc-mycute プラグインをインストールする。

```bash
make install-ecc-mycute
```

5. GitHub CLI でログインする。接続方式は SSH を選ぶこと。

```bash
make release-login
```

6. 最初のビルドを実行する。これは必ず失敗するが、後続の手順で必要なアセットを生成するために必要。

```bash
make build
```

以下のエラーが出れば正常（CORE_BIN_BYTES が参照するバイナリがまだ存在しないため）：

```
error: couldn't read `src/../target/release/mycute-server-core`: No such file or directory (os error 2)
  --> src/launcher.rs:22:32
   |
22 | static CORE_BIN_BYTES: &[u8] = include_bytes!("../target/release/mycute-server-core");
   |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

7. サーバーバイナリを単体ビルドし、上記で不足していたバイナリを生成する。

```bash
make server
```

8. 改めて完全ビルドを実行する。これが成功すればセットアップ完了。

```bash
make build
```

### 確認

`make build` が正常終了すれば、Windows でのビルド環境は整っている。

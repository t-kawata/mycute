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

### 追加で必要なツール

ビルド中に以下のツールが必要になる。事前にインストールしておくこと：

| ツール | 用途 | 入手方法 |
|--------|------|----------|
| CMake | `aws-lc-sys` / `lbug` のビルド | Build Tools に同梱、または `choco install cmake` |
| Ninja | `lbug` の cmake ビルド（generator） | `choco install ninja` または手動ダウンロード |
| LLVM（clang） | `sherpa-rs-sys` / `bindgen` の libclang バインディング | ポータブルアーカイブを手動展開 |
| GitHub CLI | リリース作業 | `choco install gh` または手動ダウンロード |

> **cmake について**: Build Tools に同梱される cmake は `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe` に配置される。`choco install cmake` で別途インストールした場合 `C:\Program Files\CMake\bin\cmake.exe` になる。いずれも**デフォルトでは PATH が通っていない**（後述の PATH 設定が必要）。

> **Ninja について**: `choco install ninja` が管理者権限を要求する場合、[GitHub Releases](https://github.com/ninja-build/ninja/releases) から `ninja-win.zip` をダウンロードし、`ninja.exe` を PATH の通ったディレクトリ（例: `%USERPROFILE%\bin`）に配置すればよい。

> **LLVM について**: 最新のポータブルアーカイブ（`clang+llvm-<version>-x86_64-pc-windows-msvc.tar.xz`）を [LLVM Releases](https://github.com/llvm/llvm-project/releases) からダウンロードし、任意のディレクトリに展開する（例: `%USERPROFILE%\llvm`）。インストーラ（`.exe`）でもよいがサイレントインストール非対応。

### 事前準備：PATH と環境変数の設定

以下の PowerShell コマンドで、ユーザー環境変数に PATH と `LIBCLANG_PATH` を恒久的に追加する（**管理者権限は不要**）。

```powershell
# cmake の PATH を通す（Build Tools 同梱版 / choco 版 のいずれかを使用している方）
$cmakePaths = @(
    "C:\Program Files\CMake\bin",
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
)
foreach ($dir in $cmakePaths) {
    if (Test-Path "$dir\cmake.exe") {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$dir*") {
            [Environment]::SetEnvironmentVariable("Path", $userPath + ";" + $dir, "User")
        }
        break
    }
}

# LLVM の bin を PATH に追加（展開先に合わせてパスを変更すること）
$llvmBin = "$env:USERPROFILE\llvm\bin"
if (Test-Path "$llvmBin\libclang.dll") {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$llvmBin*") {
        [Environment]::SetEnvironmentVariable("Path", $userPath + ";" + $llvmBin, "User")
    }
    [Environment]::SetEnvironmentVariable("LIBCLANG_PATH", $llvmBin, "User")
}
```

**Git Bash（MSYS2）ユーザー** は、上記の Windows 環境変数に加えて `~/.bashrc` にも以下を追記しておくと確実：

```bash
export PATH="$PATH:/c/Program Files/CMake/bin:/c/Users/$USERNAME/llvm/bin"
export LIBCLANG_PATH="/c/Users/$USERNAME/llvm/bin"
```

設定後、**一度ターミナルを再起動する**と変更が反映される。

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

管理者権限がない場合や Chocolatey が使えない場合は、[GitHub Releases](https://github.com/cli/cli/releases) から `gh_<version>_windows_amd64.zip` をダウンロードし、`gh.exe` を PATH の通ったディレクトリ（例: `%USERPROFILE%\bin`）に配置すればよい。

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

以下のエラーが出れば正常（CORE_BIN_BYTES が参照するバイナリがまだ存在しないため）。  
Windows ではメッセージ内のパスに `.exe` 拡張子と `\`（バックスラッシュ区切り）が含まれる点が macOS と異なる：

```
error: couldn't read `src\../target/release/mycute-server-core.exe`: 指定されたファイルが見つかりません。 (os error 2)
  --> src\launcher.rs:37:32
   |
37 | static CORE_BIN_BYTES: &[u8] = include_bytes!("../target/release/mycute-server-core.exe");
   |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
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

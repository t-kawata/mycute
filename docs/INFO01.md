Now I have a comprehensive understanding. The `is_local_url()` function checks if a URL is relative to the `devUrl`. Since in dev mode, the webview at `http://localhost:9000` IS the configured `devUrl`, it should be treated as `Origin::Local`. But the error message shows `Origin::Remote`. This confirms the bug is about something else entirely. Let me re-read the error message carefully and the source code one more time.

Looking at the `resolve_access` code again (lines 439-471 of authority.rs), and the `resolve_access_message` (lines 228-436), I now see the exact issue. The error message format `"process.exit not allowed on window \"main\", webview \"main\", URL: http://localhost:9000/"` matches the code path at line 356 where `origin.matches(&cmd.context)` succeeds (the origin matches) BUT the webview/window pattern matching fails.

Wait - but the `allowed on` list shows `webviews: "*"` which should match anything. Let me re-examine this...

Actually, looking more carefully at the `resolve_access_message` at line 342-355:

```rust
let resolved_matching_origin = resolved
    .iter()
    .filter(|cmd| origin.matches(&cmd.context))
    .collect::<Vec<&ResolvedCommand>>();
```

If this filter is **empty** (no origin matches), then it falls through to the else branch on line 356. The key question is: **is `devUrl` treated as `Origin::Local` or `Origin::Remote`?**

Based on the `is_local_url` function, if `devUrl` is `http://localhost:9000` and the webview URL is `http://localhost:9000/`, then `make_relative()` should succeed and it should be `Origin::Local`. The default capability has no `context` field, so permissions default to `ExecutionContext::Local`. This should match.

BUT - if for some reason the `is_local_url` check fails (e.g., trailing slash mismatch in `make_relative`), the origin would be `Origin::Remote` and the `ExecutionContext::Local` would not match. This IS the bug.

Now I have the complete picture. Let me compose the answer.

Tauri v2 の権限チェックのソースコードを精査した結果、根本原因が特定できました。**capability の `webviews` フィールドではなく、`context`（Origin のマッチング）が問題の核心です。**

## 根本原因: Origin マッチングの失敗

Tauri v2 の IPC 権限チェックは、**3段階**のフィルタリングを行います: [fossies](https://fossies.org/linux/www/tauri-tauri-v2.10.3.tar.gz/tauri-tauri-v2.10.3/crates/tauri/src/ipc/authority.rs)

1. **Origin（コンテキスト）**: `Local` か `Remote` かのマッチ
2. **Window ラベル**: glob パターンでのマッチ
3. **Webview ラベル**: glob パターンでのマッチ

エラーメッセージの `"not allowed on window \"main\", webview \"main\", URL: http://localhost:9000/"` は、**Origin のマッチングが失敗している**ことを示しています 。 [fossies](https://fossies.org/linux/www/tauri-tauri-v2.10.3.tar.gz/tauri-tauri-v2.10.3/crates/tauri/src/ipc/authority.rs)

### `is_local_url()` の判定ロジック

Tauri は `on_message()` で IPC リクエストを受け取る際、リクエスト元 URL が「ローカル」かどうかを `is_local_url()` で判定します。この関数は以下を順にチェックします :

1. Tauri カスタムプロトコル (`tauri://`) と一致するか
2. **`devUrl` (または `frontendDist`) からの相対パスとして解決できるか** (`make_relative()`)
3. ユーザー登録カスタムプロトコルか

`devUrl` に `http://localhost:9000` が設定されている場合、**通常は** `Origin::Local` と判定されます。しかし、`make_relative()` による URL マッチングは厳密で、**URL の微妙な差異**（末尾スラッシュ、ポート表記等）で失敗し `Origin::Remote` になることがあります。

### なぜ `allowed on` に表示されるのに拒否されるのか

capability ファイルに `context` フィールドを指定しないと、デフォルトで `ExecutionContext::Local` が使われます。`Origin::Remote` として判定されたリクエストは `ExecutionContext::Local` と一致しないため、たとえ `webviews: "*"` や `windows: "*"` が設定されていても**拒否**されます: [fossies](https://fossies.org/linux/www/tauri-tauri-v2.10.3.tar.gz/tauri-tauri-v2.10.3/crates/tauri/src/ipc/authority.rs)

```rust
// authority.rs 457-461 行目
origin.matches(&cmd.context)  // ← ここで Remote ≠ Local → false
  && (cmd.webviews.iter().any(|w| w.matches(webview))
      || cmd.windows.iter().any(|w| w.matches(window)))
```

## 解決方法

### 方法 1: `context` フィールドに `remote` を追加する（推奨）

capability に `context` フィールドを追加して、リモート URL からのアクセスも許可してください: [github](https://github.com/tauri-apps/tauri/issues/8800)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "enables the default permissions",
  "local": true,
  "remote": {
    "urls": ["http://localhost:*", "http://127.0.0.1:*"]
  },
  "windows": ["main", "overlay", "snackbar", "*"],
  "webviews": ["main", "*"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-close",
    "core:window:allow-hide",
    "core:event:default",
    "process:default",
    "process:allow-exit",
    "process:allow-restart"
  ]
}
```

重要なポイント:
- `"local": true` は暗黙のデフォルトですが、明示的に書くと明確です
- `"remote": { "urls": ["http://localhost:*"] }` で localhost からのリモートアクセスを許可します
- `webviews` フィールドは **webview のラベル名**（`"main"` 等）を指定するものであり、**URL ではありません** 。URL を `webviews` に入れても Origin のマッチングには影響しません [fossies](https://fossies.org/linux/www/tauri-tauri-v2.10.3.tar.gz/tauri-tauri-v2.10.3/crates/tauri/src/ipc/authority.rs)

### 方法 2: `tauri.conf.json` の `devUrl` を確認する

`tauri.conf.json` の `build.devUrl` と実際の開発サーバーの URL が**完全に一致**していることを確認してください:

```json
{
  "build": {
    "devUrl": "http://localhost:9000"
  }
}
```

URL が一致していれば `is_local_url()` は `true` を返し、`Origin::Local` として処理されます。末尾スラッシュの有無が影響する場合もあるため、両方試してみてください 。

### 方法 3: dev モード専用の capability ファイルを分ける

`platforms` フィールドを活用して、dev 時のみ remote context を許可する capability を作成する方法もあります: [v2.tauri](https://v2.tauri.app/learn/security/capabilities-for-windows-and-platforms/)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "dev-process",
  "description": "dev mode process permissions",
  "remote": {
    "urls": ["http://localhost:*", "http://127.0.0.1:*"]
  },
  "windows": ["*"],
  "webviews": ["*"],
  "permissions": [
    "process:allow-exit",
    "process:allow-restart"
  ]
}
```

## `webviews` フィールドの誤解について

現在の設定で `webviews` に URL を並べていますが、`webviews` フィールドは **webview のラベル名**に対する glob パターンマッチです 。エラーメッセージで `allowed on: [..., webviews: "*", "http://localhost:9000", ...]` と表示されるのは、文字列がそのまま webview ラベルの glob パターンとして格納されているだけで、URL マッチングには使われていません。URL ベースのアクセス制御は `context` の `local` / `remote` フィールドで行います 。 [github](https://github.com/tauri-apps/tauri/issues/8800)
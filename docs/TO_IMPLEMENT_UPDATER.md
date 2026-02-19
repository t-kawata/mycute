Tauri v2 なら公式の Updater プラグインを使うのが一番楽で、GitHub Releases や S3 上の static JSON／自前の更新サーバーと組み合わせて自動更新を実現できます。 [v2.tauri](https://v2.tauri.app/ja/plugin/updater/)

## 全体像

Tauri の更新はざっくり言うと次の 3 つを用意すれば動きます。 [thatgurjot](https://thatgurjot.com/til/tauri-auto-updater/)

- アップデート用のバイナリ（updater artifacts）をビルドする設定
- バージョン情報・署名・ダウンロード URL を返す更新サーバー（または static JSON）
- アプリ内で `check()` してダウンロード＆再起動する処理

以下、v2 系を前提に流れだけコンパクトに。

## 1. Updater プラグイン導入

Rust 側（`src-tauri/Cargo.toml`）にプラグインを追加します。 [github](https://github.com/tauri-apps/tauri-plugin-updater)

```toml
# デスクトップのみ
[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]
tauri-plugin-updater = "2.0.0"
tauri-plugin-process = "2.0.0"
tauri-plugin-dialog = "2.0.0" # 任意（ダイアログ出すなら）
```

`src-tauri/src/main.rs` でプラグインを登録します。 [ratulmaharaj](https://ratulmaharaj.com/posts/tauri-automatic-updates/)

```rust
fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_dialog::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
```

## 2. tauri.conf / tauri.conf.json で updater 設定

公式ドキュメントどおり `tauri.updater` を設定し、ビルド時に updater artifacts を出すようにします。 [v2.tauri](https://v2.tauri.app/ja/plugin/updater/)

代表的なポイントは:

- `active: true`（アップデート有効化）
- `endpoints: ["https://…/latest.json"]`（更新情報 JSON の URL）
- `pubkey`（Tauri が署名検証に使う公開鍵）
- `createUpdaterArtifacts: true`（または `"v1Compatible"`）

これで `tauri build` 時に、通常のインストーラ／app に加えてアップデート用のファイル群が生成されます。 [v2.tauri](https://v2.tauri.app/ja/plugin/updater/)

## 3. 更新サーバー or static JSON

最小構成なら「static JSON」をホスティングするだけで OK です。 [github](https://github.com/orgs/tauri-apps/discussions/10206)

- GitHub Releases を使う場合  
  - リリース作成時に生成された updater artifacts をアップロード
  - `latest.json` を同リポジトリや GitHub Pages／Gist などに置く
- S3／自前サーバーの場合  
  - 同じく updater artifacts を置き、`latest.json` で最新版のバージョン・署名・URL を返す

`latest.json` のフォーマットは公式ドキュメントの例に従います（OS ごとにダウンロード URL を分けられる）。 [github](https://github.com/orgs/tauri-apps/discussions/10206)

## 4. フロントエンドから更新チェック

JS 側では `@tauri-apps/plugin-updater` を使って、起動時や「アップデートを確認」ボタンでチェック → ダウンロード → 再起動、というフローを実装します。 [qiita](https://qiita.com/takavfx/items/d4033cb4e0566ed36ef2)

```ts
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

async function checkForUpdates() {
  const update = await check()
  if (!update) return

  // ここでダイアログを出してユーザーに確認してもよい
  await update.downloadAndInstall()
  await relaunch()
}
```

Svelte / React なら起動時に一度だけチェックしたり、設定画面に「アップデートを確認」ボタンを置くイメージです。 [qiita](https://qiita.com/takavfx/items/d4033cb4e0566ed36ef2)

## 5. 運用イメージ

リリースごとにやることはだいたい次の通りです。 [thatgurjot](https://thatgurjot.com/til/tauri-auto-updater/)

- バージョンを上げて `tauri build --bundles …` でビルド
- 出力された updater artifacts をホスティング先にアップロード
- `latest.json` を新バージョン情報で更新
- ユーザーのアプリは起動時 or 手動チェックで自動更新

GitHub Actions でビルド〜リリース〜`latest.json` 更新まで自動化している例も多いので、CI まで含めて設計すると楽です。 [reddit](https://www.reddit.com/r/tauri/comments/1qxvoet/shipped_a_production_macos_app_with_tauri_20/)

***

もし「GitHub Releases + static JSON でやりたい」「Mac App Store / MS Store に出していてストア更新とどう整合取るか」みたいな前提があるなら、そこ前提にもう少し踏み込んだ構成例も書きます。
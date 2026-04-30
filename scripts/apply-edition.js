#!/usr/bin/env node
// =============================================================
// apply-edition.js
// エディション設定をプロジェクトの各ファイルに適用するスクリプト。
// 使用方法: node scripts/apply-edition.js <edition-slug>
// 例: node scripts/apply-edition.js neco-asovi
// =============================================================

import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

// ESM環境での __dirname 相当の取得
const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// プロジェクトルートは scripts/ の1つ上
const ROOT = path.resolve(__dirname, '..')

// =============================================================
// 1. 引数の検証
// =============================================================
const editionSlug = process.argv[2]
if (!editionSlug) {
  console.error('Error: Edition slug is required.')
  console.error('  Usage: node scripts/apply-edition.js <edition-slug>')
  console.error('  Example: node scripts/apply-edition.js neco-asovi')
  process.exit(1)
}

// =============================================================
// 2. editions.json からエディション定義を読み込む
// =============================================================
const editionsPath = path.join(ROOT, 'editions.json')
if (!fs.existsSync(editionsPath)) {
  console.error(`Error: editions.json not found at: ${editionsPath}`)
  process.exit(1)
}

const editions = JSON.parse(fs.readFileSync(editionsPath, 'utf8'))
const edition = editions[editionSlug]

if (!edition) {
  const available = Object.keys(editions).join(', ')
  console.error(`Error: Unknown edition "${editionSlug}". Available editions: ${available}`)
  process.exit(1)
}

const { display_name, slug, identifier, data_dir, repo, icon_path } = edition
console.log(`Applying edition: ${display_name} (${slug})`)

// =============================================================
// 3. tauri.conf.json の更新
//    変更対象: productName, identifier
//    その他のフィールドは一切触れない（JSON.parseによる構造的操作）
// =============================================================
const tauriConfPath = path.join(ROOT, 'tauri.conf.json')
const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'))
tauriConf.productName = display_name
tauriConf.identifier = identifier
fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n')
console.log(`  [OK] tauri.conf.json: productName="${display_name}", identifier="${identifier}"`)

// =============================================================
// 4. Info.plist の更新
//    変更対象: CFBundleName, CFBundleIdentifier
//    正規表現による精密な置換で、XML構造を破壊しない
// =============================================================
const plistPath = path.join(ROOT, 'Info.plist')
let plistContent = fs.readFileSync(plistPath, 'utf8')

// CFBundleName の値を更新
plistContent = plistContent.replace(
  /(<key>CFBundleName<\/key>\s*<string>)[^<]*(<\/string>)/,
  `$1${slug}$2`
)
// CFBundleIdentifier の値を更新
plistContent = plistContent.replace(
  /(<key>CFBundleIdentifier<\/key>\s*<string>)[^<]*(<\/string>)/,
  `$1${identifier}$2`
)
fs.writeFileSync(plistPath, plistContent)
console.log(`  [OK] Info.plist: CFBundleName="${slug}", CFBundleIdentifier="${identifier}"`)

// =============================================================
// 5. web/package.json の更新
//    変更対象: productName, description
//    その他のフィールドは一切触れない（JSON.parseによる構造的操作）
// =============================================================
const webPkgPath = path.join(ROOT, 'web', 'package.json')
const webPkg = JSON.parse(fs.readFileSync(webPkgPath, 'utf8'))
webPkg.productName = display_name
webPkg.description = display_name
fs.writeFileSync(webPkgPath, JSON.stringify(webPkg, null, 2) + '\n')
console.log(`  [OK] web/package.json: productName="${display_name}", description="${display_name}"`)

// =============================================================
// 6. web/src/configs/settings.ts の更新
//    変更対象: APP_NAME の値のみ
//    正規表現による精密な置換
// =============================================================
const settingsPath = path.join(ROOT, 'web', 'src', 'configs', 'settings.ts')
let settingsContent = fs.readFileSync(settingsPath, 'utf8')
settingsContent = settingsContent.replace(
  /(export const APP_NAME\s*=\s*')[^']*(')/,
  `$1${display_name}$2`
)
fs.writeFileSync(settingsPath, settingsContent)
console.log(`  [OK] web/src/configs/settings.ts: APP_NAME="${display_name}"`)

// =============================================================
// 7. scripts/macos-setup.command の更新
//    変更対象: APP_PATH, 表示名
// =============================================================
const setupScriptPath = path.join(ROOT, 'scripts', 'macos-setup.command')
if (fs.existsSync(setupScriptPath)) {
  let setupContent = fs.readFileSync(setupScriptPath, 'utf8')
  setupContent = setupContent.replace(
    /APP_PATH="\/Applications\/[^"]*\.app"/,
    `APP_PATH="/Applications/${display_name}.app"`
  )
  setupContent = setupContent.replace(
    /echo " [^:]*: macOS App Setup Utility "/,
    `echo " ${display_name}: macOS App Setup Utility "`
  )
  setupContent = setupContent.replace(
    /ERROR: [^ ]*\.app not found/,
    `ERROR: ${display_name}.app not found`
  )
  setupContent = setupContent.replace(
    /Attempting to launch [^.]*\.\.\./,
    `Attempting to launch ${display_name}...`
  )
  fs.writeFileSync(setupScriptPath, setupContent)
  console.log(`  [OK] scripts/macos-setup.command: APP_PATH="/Applications/${display_name}.app"`)
}

// =============================================================
// 7. .env ファイルの生成
//    Rust のコンパイル時に参照される環境変数を書き出す。
//    このファイルは Makefile から source して使用される。
//    ※ .env はリポジトリには含めない（.gitignore 対象）
// =============================================================
const envContent = [
  `# このファイルは apply-edition.js によって自動生成されます。直接編集しないでください。`,
  `# Generated by apply-edition.js at ${new Date().toISOString()}`,
  `APP_EDITION="${slug}"`,
  `APP_SLUG="${slug}"`,
  `APP_DISPLAY_NAME="${display_name}"`,
  `APP_DATA_DIR="${data_dir}"`,
  `APP_REPO="${repo}"`,
  `APP_OSCA_PREFIX="${slug}-osca-"`,
  `APP_OSCA_PATH="/${slug}-osca.pem"`,
  `APP_SERVER_NAME="${display_name}-server"`,
  `APP_BUNDLE_ID="${identifier}"`,
  `APP_BUNDLE_PATH="/Applications/${display_name}.app"`,
  `APP_ICON_PATH="${icon_path}"`,
].join('\n') + '\n'

const envPath = path.join(ROOT, '.env')
fs.writeFileSync(envPath, envContent)
console.log(`  [OK] .env: APP_EDITION="${slug}", APP_DATA_DIR="${data_dir}", APP_REPO="${repo}"`)

// =============================================================
// 完了報告
// =============================================================
console.log(`\nEdition "${display_name}" applied successfully.`)

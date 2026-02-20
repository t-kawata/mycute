use crate::entities::{prelude::*, replace_items, replaces};
use crate::mode::rt::rtreq::replaces_req::{
    CreateReplacesReq, ImportReplacesReq, SearchReplacesReq, UpdateReplacesReq,
};
use crate::mode::rt::rtres::replace_items_res::ReplaceItemDetail;
use crate::mode::rt::rtres::replaces_res::{ExportReplacesRes, ReplacesDetail, ReplacesListItem};
use crate::utils::time::now;
use indexmap::IndexMap;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    sea_query::{Expr, Func},
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Select, Set, TransactionError, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

// デフォルト置換セットのUUID
pub const DEFAULT_REPLACE_SET_ID: &str = "00000000-0000-0000-0000-000000000001";

// settings.json から移行したハードコードされたデフォルト置換リスト
pub fn get_default_replaces() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (" false ", vec!["フォルス"]),
        (" true ", vec!["トルー"]),
        (" true か false ", vec!["トルーカフォルス"]),
        (" true または false ", vec!["トルーまたはフォルス"]),
        ("Android", vec!["アンドロイド"]),
        ("Base64", vec!["ベース64"]),
        ("C#", vec!["シーシャープ", "Cシャープ"]),
        ("CLI上", vec!["CL愛上"]),
        ("HTML", vec!["エイチティーエムエル"]),
        ("Headless", vec!["ヘッドレス"]),
        ("JSON", vec!["ジェイソン"]),
        (
            "JavaScript",
            vec![
                "Javaスクリップス",
                "Javaスクリプト",
                "ジャバスクリプト",
                "ヤバスクリプト",
            ],
        ),
        (
            "MYCUTE",
            vec![
                "My cute",
                "My cut",
                "マイcute",
                "マイCute",
                "マイキュート",
                "マキュート",
                "マイキーと",
                "マイcut",
                "マイCut",
                "マイ級と",
                "マイQと",
                "迷宮と",
            ],
        ),
        (
            "Makefile",
            vec![
                "makefile",
                "makeファイル",
                "Makeファイル",
                "メイクファイル",
                "メークファイル",
            ],
        ),
        ("MySQL", vec!["My SQL", "マイSQL"]),
        ("NG", vec!["Ng"]),
        (
            "OK",
            vec!["オーケー", "おーけー", "オッケー", "おっけー", "Ok"],
        ),
        (
            "OpenAI",
            vec![
                "オープンエーアイ",
                "Open AI",
                "オープンエアー",
                "オープンAI",
                "オープンエア",
            ],
        ),
        ("Quasar", vec!["クエーサー"]),
        ("README", vec!["リードmini", "Leadミー", "リードミー"]),
        (
            "README.md",
            vec![
                "リードミードットMD",
                "リード、Me MD",
                "リードMe.MD",
                "リードミー.md",
                "リードミー.MD",
                "リードミーMD",
            ],
        ),
        (
            "REST API",
            vec![
                "レストエーピーアイ",
                "レスト、API",
                "ベスト、API",
                "ベストAPI",
                "レストAPI",
            ],
        ),
        ("Rust", vec!["ラスト"]),
        ("Swagger", vec!["諏訪側"]),
        (
            "Tauri",
            vec![
                "タウ",
                "Power BI",
                "パウリー",
                "ハウリー",
                "タウリン",
                "タウル",
                "タウリ",
                "タウり",
                "たうり",
                "パウリ",
                "ハウリ",
            ],
        ),
        ("Tauriの", vec!["タウの"]),
        ("TypeScript", vec!["タイプスクリプト"]),
        ("VDR", vec!["BD-R", "VD R"]),
        ("WebView", vec!["ウェブビュー"]),
        ("Windows", vec!["ウィンドウズ"]),
        ("apps", vec!["アップス"]),
        ("enum", vec!["言いナム", "イーナム", "イナム"]),
        ("iframe", vec!["アイフレーム"]),
        ("localhost", vec!["ローカルホスト"]),
        ("という", vec!["と、いう", "という。", "と言う"]),
        ("インターセプト", vec!["インターセット"]),
        ("エンドポイント", vec!["&ポイント"]),
        (
            "クレート",
            vec!["グレート", "クレイト", "クレーと", "クレイと"],
        ),
        ("コマンドライン", vec!["コマンドLINE"]),
        ("コンポーネント", vec!["梱包メント", "梱包ネント"]),
        ("ステップ名", vec!["ステップメイ"]),
        (
            "ダミー実装",
            vec![
                "ダヴィンチ疾走",
                "ダヴィンチ失踪",
                "ダメージ走",
                "ダメージ層",
            ],
        ),
        ("トレイト", vec!["トレイと", "トレート", "トレーと"]),
        ("ネスト", vec!["ネスと"]),
        ("ノード", vec!["濃度", "脳度"]),
        ("フェーズ", vec!["フェイズ"]),
        ("プロキシ", vec!["プロ棋士"]),
        ("一時ディレクトリ", vec!["1時ディレクトリ"]),
        ("一部", vec!["1部"]),
        ("任命証", vec!["任命症"]),
        ("作業", vec!["詐欺"]),
        ("全件取得", vec!["全権取得"]),
        ("公開鍵", vec!["公開カギ"]),
        ("参照", vec!["山椒"]),
        ("同期リクエスト", vec!["動機リクエスト"]),
        ("固定値", vec!["コテージ"]),
        ("型定義", vec!["片定義", "片手技", "型定期"]),
        ("完全修飾", vec!["完全就職"]),
        ("実体", vec!["実態"]),
        ("実装", vec!["自走"]),
        ("実装計画", vec!["自走計画"]),
        ("実装計画書", vec!["実装、計画書"]),
        ("導出", vec!["同出"]),
        ("川田", vec!["川畑", "川端", "桑田", "河田"]),
        ("川田です", vec!["変わったです"]),
        ("強制", vec!["矯正"]),
        ("描画", vec!["茗荷"]),
        ("改修", vec!["回収"]),
        ("明示", vec!["明治"]),
        ("書き入れ", vec!["下記入れ"]),
        ("末尾", vec!["松尾"]),
        ("相対", vec!["早退"]),
        ("等々", vec!["等等"]),
        ("網羅", vec!["モーラ"]),
        ("置換", vec!["チカン", "痴漢"]),
        ("自前", vec!["時前"]),
        ("設け", vec!["儲け"]),
        ("設定値", vec!["設定地"]),
        ("認証局", vec!["認証曲"]),
        ("起動", vec!["軌道"]),
        ("透過", vec!["投下"]),
        ("階層構造", vec!["改装構造"]),
        ("静的", vec!["性的"]),
    ]
}

/// デフォルトの置換リストをDBにシードする（存在しない場合）
pub async fn seed_default_replaces<C>(db: &C) -> Result<(), DbErr>
where
    C: ConnectionTrait + TransactionTrait,
{
    let uuid = Uuid::parse_str(DEFAULT_REPLACE_SET_ID).unwrap();

    // 存在確認
    if Replaces::find_by_id(uuid).one(db).await?.is_some() {
        log::info!("Default replaces already seeded.");
        return Ok(());
    }

    log::info!("Seeding default replaces...");

    // トランザクション内で実行 (クロージャ形式)
    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            let now = now();
            // セットの作成
            let set = replaces::ActiveModel {
                id: Set(uuid),
                apx_id: Set(0),
                vdr_id: Set(0),
                name: Set("Default Dictionary".to_string()),
                description: Set(Some("Standard replacements for MYCUTE".to_string())),
                is_active: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
            };
            set.insert(txn).await?;
            log::info!("Seeded replace set: {}", uuid);

            // アイテムの作成
            let items = get_default_replaces();
            let mut active_items = Vec::new();

            for (rank, (key, texts)) in items.into_iter().enumerate() {
                let texts_json = json!(texts);
                active_items.push(replace_items::ActiveModel {
                    id: NotSet,
                    replace_id: Set(uuid),
                    apx_id: Set(0),
                    vdr_id: Set(0),
                    key: Set(key.to_string()),
                    texts: Set(texts_json),
                    rank: Set(rank as i32),
                    created_at: Set(now),
                    updated_at: Set(now),
                });
            }

            // SQLite の変数制限制限を避けるため、50件ずつのチャンクで挿入
            if !active_items.is_empty() {
                for chunk in active_items.chunks(50) {
                    ReplaceItems::insert_many(chunk.to_vec()).exec(txn).await?;
                }
                log::info!("Seeded items.");
            }

            Ok(())
        })
    })
    .await
    .map_err(|e| match e {
        TransactionError::Connection(e) => e,
        TransactionError::Transaction(e) => e,
    })?;

    Ok(())
}

/// 全てのアクティブな置換セットを結合した IndexMap と、アクティブなセットIDリストを取得する。
pub async fn get_active_replaces_map<C>(
    db: &C,
) -> Result<(IndexMap<String, Vec<String>>, Vec<Uuid>), DbErr>
where
    C: ConnectionTrait,
{
    // 全てのアクティブなセットを検索
    // 備考: ローカルクライアント用途では、apx/vdrに関わらず全てのアクティブセットを読み込む（通常は 0/0）。
    // もし厳密なマルチテナントフィルタリングが必要な場合は、引数を追加する必要がある。
    let active_sets: Vec<replaces::Model> = Replaces::find()
        .filter(replaces::Column::IsActive.eq(true))
        .all(db)
        .await?;

    if active_sets.is_empty() {
        return Ok((IndexMap::new(), Vec::new()));
    }

    let set_ids: Vec<Uuid> = active_sets.iter().map(|s| s.id).collect();

    // アクティブなセットに属する全アイテムを取得。Rank順（Rankが大きいほど後から適用されることを想定）。
    // 置換マップでは順番が重要。IndexMapは挿入順序を保持する。
    // 決定論的な順序を保証するため、Rankでソートする。
    let items: Vec<replace_items::Model> = ReplaceItems::find()
        .filter(replace_items::Column::ReplaceId.is_in(set_ids.clone()))
        .order_by_asc(replace_items::Column::Rank) // 0, 1, 2...
        .all(db)
        .await?;

    let mut map = IndexMap::new();
    for item in items {
        if let Ok(vec_str) = serde_json::from_value::<Vec<String>>(item.texts) {
            map.insert(item.key, vec_str);
        }
    }

    Ok((map, set_ids))
}

// ===========================================
// Replaces Logic
// ===========================================

fn find_replaces_base(apx_id: u32, vdr_id: u32) -> Select<Replaces> {
    Replaces::find()
        .filter(replaces::Column::ApxId.eq(apx_id))
        .filter(replaces::Column::VdrId.eq(vdr_id))
}

pub async fn search_replaces<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    req: SearchReplacesReq,
) -> Result<(Vec<ReplacesListItem>, u64), DbErr>
where
    C: ConnectionTrait,
{
    let mut query = find_replaces_base(apx_id, vdr_id);

    if let Some(k) = req.keyword {
        let lower_k = format!("%{}%", k.to_lowercase());
        query = query.filter(
            Condition::any()
                .add(
                    Expr::expr(Func::lower(Expr::col(replaces::Column::Name)))
                        .like(lower_k.clone()),
                )
                .add(
                    Expr::expr(Func::lower(Expr::col(replaces::Column::Description))).like(lower_k),
                ),
        );
    }
    if let Some(active) = req.is_active {
        query = query.filter(replaces::Column::IsActive.eq(active));
    }

    let paginator = query
        .order_by_desc(replaces::Column::UpdatedAt)
        .paginate(db, req.limit);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(req.offset).await?;

    let list = items
        .into_iter()
        .map(|m| ReplacesListItem {
            id: m.id,
            name: m.name,
            description: m.description,
            is_active: m.is_active,
            updated_at: m.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    Ok((list, total))
}

pub async fn get_replaces<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    id: Uuid,
) -> Result<Option<(ReplacesDetail, u64)>, DbErr>
where
    C: ConnectionTrait,
{
    let set = find_replaces_base(apx_id, vdr_id)
        .filter(replaces::Column::Id.eq(id))
        .one(db)
        .await?;

    if let Some(m) = set {
        let items_count = ReplaceItems::find()
            .filter(replace_items::Column::ReplaceId.eq(id))
            .count(db)
            .await?;

        Ok(Some((
            ReplacesDetail {
                id: m.id,
                name: m.name,
                description: m.description,
                is_active: m.is_active,
                created_at: m.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                updated_at: m.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            },
            items_count,
        )))
    } else {
        Ok(None)
    }
}

pub async fn create_replaces<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    req: CreateReplacesReq,
) -> Result<Uuid, DbErr>
where
    C: ConnectionTrait,
{
    let now = now();
    let uuid = Uuid::new_v4();

    let active = replaces::ActiveModel {
        id: Set(uuid),
        apx_id: Set(apx_id as i32),
        vdr_id: Set(vdr_id as i32),
        name: Set(req.name),
        description: Set(req.description),
        is_active: Set(false), // 初期状態は非アクティブ
        created_at: Set(now),
        updated_at: Set(now),
    };

    active.insert(db).await.map(|m| m.id)
}

pub async fn update_replaces<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    id: Uuid,
    req: UpdateReplacesReq,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let target = find_replaces_base(apx_id, vdr_id)
        .filter(replaces::Column::Id.eq(id))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Replaces {} not found", id)))?;

    let mut active: replaces::ActiveModel = target.into();
    if let Some(n) = req.name {
        active.name = Set(n);
    }
    if let Some(d) = req.description {
        active.description = Set(Some(d));
    }
    active.updated_at = Set(now());
    active.update(db).await?;
    Ok(())
}

pub async fn set_replace_set_active<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    id: Uuid,
    is_active: bool,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let target = find_replaces_base(apx_id, vdr_id)
        .filter(replaces::Column::Id.eq(id))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Replaces {} not found", id)))?;

    let mut active: replaces::ActiveModel = target.into();
    active.is_active = Set(is_active);
    active.updated_at = Set(now());
    active.update(db).await?;
    Ok(())
}

pub async fn delete_replaces<C>(db: &C, apx_id: u32, vdr_id: u32, id: Uuid) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let target = find_replaces_base(apx_id, vdr_id)
        .filter(replaces::Column::Id.eq(id))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Replaces {} not found", id)))?;

    // アイテムの連鎖削除は通常DBの外部キー制約で処理されるが、ここでは明示的に行うことで安全性を高める
    ReplaceItems::delete_many()
        .filter(replace_items::Column::ReplaceId.eq(id))
        .exec(db)
        .await?;

    target.delete(db).await?;
    Ok(())
}

// ===========================================
// Export / Import
// ===========================================

pub async fn export_replaces<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    id: Uuid,
) -> Result<ExportReplacesRes, DbErr>
where
    C: ConnectionTrait,
{
    // 詳細を取得
    let (detail, _) = get_replaces(db, apx_id, vdr_id, id)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Replaces {} not found", id)))?;

    // 全アイテムを取得
    let items = ReplaceItems::find()
        .filter(replace_items::Column::ReplaceId.eq(id))
        .filter(replace_items::Column::ApxId.eq(apx_id))
        .filter(replace_items::Column::VdrId.eq(vdr_id))
        .order_by_asc(replace_items::Column::Rank)
        .all(db)
        .await?;

    let item_details = items
        .into_iter()
        .map(|m| {
            let texts_vec: Vec<String> = serde_json::from_value(m.texts).unwrap_or_default();
            ReplaceItemDetail {
                id: m.id,
                replace_id: m.replace_id,
                key: m.key,
                texts: texts_vec,
                rank: m.rank,
            }
        })
        .collect();

    Ok(ExportReplacesRes {
        replace: detail,
        items: item_details,
    })
}

pub async fn import_replaces<C>(
    db: &C,
    apx_id: u32,
    vdr_id: u32,
    req: ImportReplacesReq,
) -> Result<Uuid, DbErr>
where
    C: TransactionTrait,
{
    let txn = db.begin().await?;

    // セットの作成
    let now = now();
    let uuid = Uuid::new_v4();

    let set = replaces::ActiveModel {
        id: Set(uuid),
        apx_id: Set(apx_id as i32),
        vdr_id: Set(vdr_id as i32),
        name: Set(req.name),
        description: Set(req.description),
        is_active: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    set.insert(&txn).await?;

    // アイテムの作成
    if !req.items.is_empty() {
        let actives: Vec<replace_items::ActiveModel> = req
            .items
            .into_iter()
            .map(|item| replace_items::ActiveModel {
                id: NotSet,
                replace_id: Set(uuid),
                apx_id: Set(apx_id as i32),
                vdr_id: Set(vdr_id as i32),
                key: Set(item.key),
                texts: Set(json!(item.texts)),
                rank: Set(item.rank),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect();
        ReplaceItems::insert_many(actives).exec(&txn).await?;
    }

    txn.commit().await?;
    Ok(uuid)
}

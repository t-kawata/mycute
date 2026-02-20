use crate::constants::{ST_BAD_REQUEST, ST_FORBIDDEN, ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND};
use crate::entities::{
    badges, belongs, cryptos, flushes, jobs, match_statuses, matches, payments, payouts, points,
    pools, usr_badges, usrs, works,
};
use crate::enums::usrtype::UsrType;
use crate::mode::rt::rterr::rterr;
use crate::mode::rt::rtreq::usrs_req::{CreateUsrReq, SearchUsrsReq, UpdateUsrReq};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::usrs_res::{
    CreateUsrRes, DehireUsrRes, DeleteUsrRes, GetUsrRes, HireUsrRes, SearchUsrsRes,
    SearchUsrsResItem, UpdateUsrRes,
};
use crate::utils::jwt::{JwtIDs, JwtRole, JwtUsr};
use crate::utils::{crypto, db::str_to_datetime};
use chrono::NaiveDateTime;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel,
    ModelTrait, QueryFilter, QuerySelect, Select, Set, TransactionTrait,
};

// ============================================================
// Private Helper for Search and Get
// ============================================================
/// 権限に基づいた共通のクエリベースを作成する
async fn find_usrs_base(ju: &JwtUsr, ids: &JwtIDs) -> Result<Select<usrs::Entity>, ApiError> {
    let query = usrs::Entity::find();
    // 権限に基づくフィルタリング
    // IDs (JwtIDs) は既にロールに応じて正規化されている。
    // apx_id と vdr_id は完全なパーティションとして扱うため、
    // VDR/USR ロールでは常に両方の条件を含める。
    match ju.role() {
        JwtRole::BD => {
            log::debug!("<UsrBl> find_usrs_base: BD role. No filtering.");
            Ok(query)
        }
        JwtRole::APX => {
            log::debug!(
                "<UsrBl> find_usrs_base: APX role. Filter apx_id: {}",
                ids.apx_id
            );
            Ok(query.filter(usrs::Column::ApxId.eq(ids.apx_id)))
        }
        JwtRole::VDR => {
            log::debug!(
                "<UsrBl> find_usrs_base: VDR role. Filter apx_id: {}, vdr_id: {}",
                ids.apx_id,
                ids.vdr_id
            );
            Ok(query
                .filter(usrs::Column::ApxId.eq(ids.apx_id))
                .filter(usrs::Column::VdrId.eq(ids.vdr_id)))
        }
        JwtRole::USR => {
            log::debug!(
                "<UsrBl> find_usrs_base: USR role. Filter apx_id: {}, vdr_id: {}, usr_id: {}",
                ids.apx_id,
                ids.vdr_id,
                ids.usr_id
            );
            Ok(query
                .filter(usrs::Column::ApxId.eq(ids.apx_id))
                .filter(usrs::Column::VdrId.eq(ids.vdr_id))
                .filter(usrs::Column::Id.eq(ids.usr_id)))
        }
    }
}

// ============================================================
// Search
// ============================================================
pub async fn search_usrs(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: SearchUsrsReq,
) -> Result<SearchUsrsRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    log::debug!("<UsrBl> search_usrs: Constructing base query.");
    let mut query = find_usrs_base(ju, ids).await?;
    // --------------------------------
    // 2. 検索条件（LIKE検索）
    // --------------------------------
    if !req.name.is_empty() {
        log::debug!("<UsrBl> search_usrs: Filter by name: {}", req.name);
        query = query.filter(usrs::Column::Name.contains(&req.name));
    }
    if !req.email.is_empty() {
        log::debug!("<UsrBl> search_usrs: Filter by email: {}", req.email);
        query = query.filter(usrs::Column::Email.contains(&req.email));
    }
    // --------------------------------
    // 3. 日時範囲のフィルタリング
    // --------------------------------
    let req_bgn_at = req.bgn_at;
    let req_end_at = req.end_at;
    log::debug!(
        "<UsrBl> search_usrs: Filter by range [{}, {}]",
        req_bgn_at,
        req_end_at
    );
    let format = "%Y-%m-%dT%H:%M:%S";
    let bgn_at = NaiveDateTime::parse_from_str(&req_bgn_at, format).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            format!("Invalid bgn_at: {}", e),
        )
    })?;
    let end_at = NaiveDateTime::parse_from_str(&req_end_at, format).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            format!("Invalid end_at: {}", e),
        )
    })?;
    // モデルの [BgnAt, EndAt] が [req.bgn_at, req.end_at] と重なるものを抽出
    query = query
        .filter(usrs::Column::BgnAt.lte(end_at))
        .filter(usrs::Column::EndAt.gte(bgn_at));
    // --------------------------------
    // 4. データの取得
    // --------------------------------
    log::debug!(
        "<UsrBl> search_usrs: Fetching records. limit: {}, offset: {}",
        req.limit,
        req.offset
    );
    let models = query
        .offset(req.offset as u64)
        .limit(req.limit as u64)
        .all(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Search query error: {}", e),
            )
        })?;
    log::debug!("<UsrBl> search_usrs: Found {} records.", models.len());
    // --------------------------------
    // 5. DBデータのレスポンス用変換
    // --------------------------------
    let usrs = models.into_iter().map(SearchUsrsResItem::from).collect();
    // --------------------------------
    // 6. 最終レスポンス
    // --------------------------------
    Ok(SearchUsrsRes { usrs })
}

// ============================================================
// Get
// ============================================================
pub async fn get_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<GetUsrRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    log::debug!("<UsrBl> get_usr: Fetching user: {}", target_usr_id);
    let query = find_usrs_base(ju, ids).await?;
    // --------------------------------
    // 2. ユーザーの取得
    // --------------------------------
    let model = query
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch user error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(ST_NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found.")
        })?;
    Ok(GetUsrRes::from(model))
}

// ============================================================
// Create
// ============================================================
pub async fn create_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: CreateUsrReq,
) -> Result<CreateUsrRes, ApiError> {
    // --------------------------------
    // 1. ロールに基づくパラメータバリデーションと初期値設定
    // --------------------------------
    let aid: Option<u32>;
    let vid: Option<u32>;
    let utype: u8;
    let target_label: &str;

    log::debug!(
        "<UsrBl> create_usr: Role-based validation for {:?}.",
        ju.role()
    );
    match ju.role() {
        JwtRole::BD => {
            // BD は APX のみ作成可能
            aid = None; // 新しい APX なので apx_id は空
            vid = None;
            utype = UsrType::Corp as u8; // APX は常に法人タイプ
            target_label = "APX";
            // 不要な項目があればエラー
            if req.usr_type.is_some()
                || req.base_point.is_some()
                || req.belong_rate.is_some()
                || req.max_works.is_some()
                || req.flush_days.is_some()
                || req.rate.is_some()
                || req.flush_fee_rate.is_some()
            {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    rterr::ERR_INVALID_REQUEST,
                    "BD can only create APX. Unnecessary parameters provided.",
                ));
            }
        }
        JwtRole::APX => {
            // APX は配下に VDR のみ作成可能
            aid = Some(ids.apx_id);
            vid = None; // 新しい VDR なので vdr_id は空
            utype = UsrType::Corp as u8; // VDR は常に法人タイプ
            target_label = "VDR";
            // VDR 必須項目のチェック
            if req.base_point.is_none()
                || req.belong_rate.is_none()
                || req.max_works.is_none()
                || req.flush_fee_rate.is_none()
            {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    rterr::ERR_INVALID_REQUEST,
                    "VDR requires base_point, belong_rate, max_works, and flush_fee_rate.",
                ));
            }
            // 不要な項目があればエラー
            if req.usr_type.is_some() || req.flush_days.is_some() || req.rate.is_some() {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    rterr::ERR_INVALID_REQUEST,
                    "APX can only create VDR. Unnecessary parameters provided.",
                ));
            }
        }
        JwtRole::VDR => {
            // VDR は配下に USR (個人/法人) を作成可能
            aid = Some(ids.apx_id);
            vid = Some(ids.vdr_id);
            target_label = "USR";
            // type は必須
            let t = req.usr_type.ok_or_else(|| {
                ApiError::new_system(
                    ST_BAD_REQUEST,
                    rterr::ERR_INVALID_REQUEST,
                    "Usr type is required.",
                )
            })?;
            utype = t;
            // 不要な項目のチェック
            if req.base_point.is_some()
                || req.belong_rate.is_some()
                || req.max_works.is_some()
                || req.flush_fee_rate.is_some()
            {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    rterr::ERR_INVALID_REQUEST,
                    "VDR cannot set base_point, belong_rate, max_works, or flush_fee_rate for USR.",
                ));
            }
            if utype == UsrType::Corp as u8 {
                // 法人としての必須項目
                if req.flush_days.is_none() || req.rate.is_none() {
                    return Err(ApiError::new_system(
                        ST_BAD_REQUEST,
                        rterr::ERR_INVALID_REQUEST,
                        "Corporate USR requires flush_days and rate.",
                    ));
                }
            } else if utype == UsrType::Indi as u8 {
                // 個人としてのチェック (不要な項目)
                if req.flush_days.is_some() || req.rate.is_some() {
                    return Err(ApiError::new_system(
                        ST_BAD_REQUEST,
                        rterr::ERR_INVALID_REQUEST,
                        "Personal USR cannot have flush_days or rate.",
                    ));
                }
            }
        }
        JwtRole::USR => {
            return Err(ApiError::new_system(
                ST_FORBIDDEN,
                rterr::ERR_AUTH,
                "USR is not allowed to create users.",
            ));
        }
    }
    // --------------------------------
    // 2. メールアドレスの重複チェック (パーティション内)
    // --------------------------------
    let req_email = req.email;
    log::debug!(
        "<UsrBl> create_usr: Email duplication check for {}.",
        req_email
    );
    let exists = usrs::Entity::find()
        .filter(usrs::Column::Email.eq(&req_email))
        .filter(usrs::Column::ApxId.eq(aid))
        .filter(usrs::Column::VdrId.eq(vid))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Email check error: {}", e),
            )
        })?;
    if exists.is_some() {
        log::debug!("<UsrBl> create_usr: Email {} already exists.", req_email);
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            format!("Email already exists as {}.", target_label),
        ));
    }
    // --------------------------------
    // 3. 名前の正規化 (個人タイプの場合)
    // --------------------------------
    let req_name = req.name;
    log::debug!("<UsrBl> create_usr: Normalizing name for type {}.", utype);
    let mut name = req_name.clone();
    if utype == UsrType::Indi as u8 {
        name = name.replace('　', " ");
        while name.contains("  ") {
            name = name.replace("  ", " ");
        }
        name = name.trim().to_string();
        if !name.contains(' ') {
            return Err(ApiError::new_system(
                ST_BAD_REQUEST,
                rterr::ERR_INVALID_REQUEST,
                "Personal name must contain a space between first and last name.",
            ));
        }
    }
    // --------------------------------
    // 4. パスワードハッシュ化
    // --------------------------------
    let req_password = req.password;
    let hashed_pw = crypto::get_hash_with_cost(&req_password, 10).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_UNEXPECTED,
            format!("Password hash error: {}", e),
        )
    })?;
    // --------------------------------
    // 5. 日時変換
    // --------------------------------
    let req_bgn_at = req.bgn_at;
    let req_end_at = req.end_at;
    let bgn_at = str_to_datetime(&req_bgn_at).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            format!("Invalid bgn_at: {}", e),
        )
    })?;
    let end_at = str_to_datetime(&req_end_at).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            format!("Invalid end_at: {}", e),
        )
    })?;
    // --------------------------------
    // 6. ActiveModel 作成と保存 (Transaction)
    // --------------------------------
    let is_vdr_creation = ju.role() == JwtRole::APX;
    log::debug!(
        "<UsrBl> create_usr: Starting transaction. is_vdr_creation: {}",
        is_vdr_creation
    );
    let created_id = conn
        .transaction::<_, u32, ApiError>(|tx| {
            Box::pin(async move {
                log::debug!("<UsrBl> create_usr: Inserting user record.");
                let mut active: usrs::ActiveModel = Default::default();
                active.apx_id = Set(aid.map(|x| x as i32));
                active.vdr_id = Set(vid.map(|x| x as i32));
                active.name = Set(name);
                active.email = Set(req_email);
                active.password = Set(hashed_pw);
                active.bgn_at = bgn_at;
                active.end_at = end_at;
                active.r#type = Set(utype as i8);
                active.base_point = Set(req.base_point.unwrap_or(0) as i32);
                active.belong_rate =
                    Set(Decimal::from_f64(req.belong_rate.unwrap_or(0.0)).unwrap_or_default());
                active.max_works = Set(req.max_works.unwrap_or(0) as i32);
                active.flush_days = Set(req.flush_days.unwrap_or(0) as i32);
                active.rate = Set(Decimal::from_f64(req.rate.unwrap_or(0.0)).unwrap_or_default());
                active.flush_fee_rate =
                    Set(Decimal::from_f64(req.flush_fee_rate.unwrap_or(0.0)).unwrap_or_default());
                let res: usrs::Model = active.insert(tx).await.map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        format!("Insert user error: {}", e),
                    )
                })?;
                // VDR作成時のみ Pool を作成
                if is_vdr_creation {
                    log::debug!("<UsrBl> create_usr: Creating pool for VDR.");
                    let pool = pools::ActiveModel {
                        apx_id: Set(aid.unwrap_or(0) as i32),
                        vdr_id: Set(res.id as i32),
                        remain: Set(0),
                        total_in: Set(0),
                        total_out: Set(0),
                        ..Default::default()
                    };
                    pool.insert(tx).await.map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Insert pool error: {}", e),
                        )
                    })?;
                }
                log::debug!("<UsrBl> create_usr: Transaction success. ID: {}", res.id);
                Ok(res.id as u32)
            })
        })
        .await?;
    // --------------------------------
    // 7. 最終レスポンス
    // --------------------------------
    Ok(CreateUsrRes { id: created_id })
}

// ============================================================
// Update
// ============================================================
pub async fn update_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
    req: UpdateUsrReq,
) -> Result<UpdateUsrRes, ApiError> {
    log::debug!(
        "<UsrBl> update_usr: Fetching target user: {}",
        target_usr_id
    );
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認
    // --------------------------------
    let model = find_usrs_base(ju, ids)
        .await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch user error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(ST_NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found.")
        })?;
    log::debug!("<UsrBl> update_usr: Found target user. Processing field updates.");
    // --------------------------------
    // 2. 更新用 ActiveModel の準備
    // --------------------------------
    let mut active: usrs::ActiveModel = model.clone().into_active_model();
    // --------------------------------
    // 3. 各フィールドの更新
    // --------------------------------
    // Type (usr_type)
    let current_type = req.usr_type.unwrap_or(model.r#type as u8);
    if let Some(t) = req.usr_type {
        active.r#type = Set(t as i8);
    }
    // Name (個人 type=2 の場合はスペースチェック)
    if let Some(mut name) = req.name {
        if current_type == 2 {
            // 全角スペースを半角に変換
            name = name.replace('　', " ");
            // 連続するスペースを1つに
            while name.contains("  ") {
                name = name.replace("  ", " ");
            }
            name = name.trim().to_string();
            // 姓名の間にスペースが必須
            if !name.contains(' ') {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    rterr::ERR_INVALID_REQUEST,
                    "Personal name must contain a space between first and last name.",
                ));
            }
        }
        active.name = Set(name);
    }
    if let Some(email) = req.email {
        active.email = Set(email);
    }
    if let Some(password) = req.password {
        let hashed = crypto::get_hash_with_cost(&password, 10).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_UNEXPECTED,
                format!("Password hash error: {}", e),
            )
        })?;
        active.password = Set(hashed);
    }
    if let Some(bgn_at) = req.bgn_at {
        active.bgn_at = str_to_datetime(&bgn_at).map_err(|e| {
            ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, e.to_string())
        })?;
    }
    if let Some(end_at) = req.end_at {
        active.end_at = str_to_datetime(&end_at).map_err(|e| {
            ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, e.to_string())
        })?;
    }
    // VDR/法人 関連項目
    if let Some(v) = req.base_point {
        active.base_point = Set(v as i32);
    }
    if let Some(v) = req.belong_rate {
        active.belong_rate = Set(Decimal::from_f64(v).ok_or_else(|| {
            ApiError::new_system(
                ST_BAD_REQUEST,
                rterr::ERR_INVALID_REQUEST,
                "Invalid belong_rate",
            )
        })?);
    }
    if let Some(v) = req.max_works {
        active.max_works = Set(v as i32);
    }
    if let Some(v) = req.flush_days {
        active.flush_days = Set(v as i32);
    }
    if let Some(v) = req.rate {
        active.rate = Set(Decimal::from_f64(v).ok_or_else(|| {
            ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid rate")
        })?);
    }
    if let Some(v) = req.flush_fee_rate {
        active.flush_fee_rate = Set(Decimal::from_f64(v).ok_or_else(|| {
            ApiError::new_system(
                ST_BAD_REQUEST,
                rterr::ERR_INVALID_REQUEST,
                "Invalid flush_fee_rate",
            )
        })?);
    }
    // --------------------------------
    // 4. 保存
    // --------------------------------
    log::debug!("<UsrBl> update_usr: Saving changes to DB.");
    active.update(conn).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Update user error: {}", e),
        )
    })?;
    log::debug!("<UsrBl> update_usr: Success.");
    // --------------------------------
    // 5. 最終レスポンス
    // --------------------------------
    Ok(UpdateUsrRes { id: target_usr_id })
}

// ============================================================
// Delete
// ============================================================
pub async fn delete_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<DeleteUsrRes, ApiError> {
    log::debug!(
        "<UsrBl> delete_usr: Fetching target user: {}",
        target_usr_id
    );
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認
    // --------------------------------
    let model = find_usrs_base(ju, ids)
        .await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch user error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(ST_NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found.")
        })?;
    log::debug!("<UsrBl> delete_usr: Found target user. Starting deletion transaction.");
    // --------------------------------
    // 2. 削除の実行
    // --------------------------------
    conn.transaction::<_, (), ApiError>(|tx| {
        Box::pin(async move {
            let target_id = model.id as i32;
            if model.apx_id.is_some() && model.vdr_id.is_none() {
                log::debug!("<UsrBl> delete_usr: Target is VDR. Cascading sub-records deletion.");
                // (1) VDR だった場合の一括削除
                let vid = target_id;
                usrs::Entity::delete_many()
                    .filter(usrs::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete sub-usrs error: {}", e),
                        )
                    })?;
                jobs::Entity::delete_many()
                    .filter(jobs::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete jobs error: {}", e),
                        )
                    })?;
                matches::Entity::delete_many()
                    .filter(matches::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete matches error: {}", e),
                        )
                    })?;
                match_statuses::Entity::delete_many()
                    .filter(match_statuses::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete match_statuses error: {}", e),
                        )
                    })?;
                works::Entity::delete_many()
                    .filter(works::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete works error: {}", e),
                        )
                    })?;
                belongs::Entity::delete_many()
                    .filter(belongs::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete belongs error: {}", e),
                        )
                    })?;
                badges::Entity::delete_many()
                    .filter(badges::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete badges error: {}", e),
                        )
                    })?;
                usr_badges::Entity::delete_many()
                    .filter(usr_badges::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete usr_badges error: {}", e),
                        )
                    })?;
                points::Entity::delete_many()
                    .filter(points::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete points error: {}", e),
                        )
                    })?;
                payments::Entity::delete_many()
                    .filter(payments::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete payments error: {}", e),
                        )
                    })?;
                pools::Entity::delete_many()
                    .filter(pools::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete pools error: {}", e),
                        )
                    })?;
                flushes::Entity::delete_many()
                    .filter(flushes::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete flushes error: {}", e),
                        )
                    })?;
                payouts::Entity::delete_many()
                    .filter(payouts::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete payouts error: {}", e),
                        )
                    })?;
                cryptos::Entity::delete_many()
                    .filter(cryptos::Column::VdrId.eq(vid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete cryptos error: {}", e),
                        )
                    })?;
            } else if model.apx_id.is_some() && model.vdr_id.is_some() {
                log::debug!("<UsrBl> delete_usr: Target is USR. Cascading sub-records deletion.");
                // (2) USR だった場合の一括削除
                let uid = target_id;
                // matches (from, to)
                matches::Entity::delete_many()
                    .filter(
                        Condition::any()
                            .add(matches::Column::From.eq(uid))
                            .add(matches::Column::To.eq(uid)),
                    )
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete matches error: {}", e),
                        )
                    })?;
                // match_statuses (from, to)
                match_statuses::Entity::delete_many()
                    .filter(
                        Condition::any()
                            .add(match_statuses::Column::From.eq(uid))
                            .add(match_statuses::Column::To.eq(uid)),
                    )
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete match_statuses error: {}", e),
                        )
                    })?;
                // works (from, to)
                works::Entity::delete_many()
                    .filter(
                        Condition::any()
                            .add(works::Column::From.eq(uid))
                            .add(works::Column::To.eq(uid)),
                    )
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete works error: {}", e),
                        )
                    })?;
                // belongs (corp_id, usr_id)
                belongs::Entity::delete_many()
                    .filter(
                        Condition::any()
                            .add(belongs::Column::CorpId.eq(uid))
                            .add(belongs::Column::UsrId.eq(uid)),
                    )
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete belongs error: {}", e),
                        )
                    })?;
                // usr_badges (corp_id, from, to)
                usr_badges::Entity::delete_many()
                    .filter(
                        Condition::any()
                            .add(usr_badges::Column::CorpId.eq(uid))
                            .add(usr_badges::Column::From.eq(uid))
                            .add(usr_badges::Column::To.eq(uid)),
                    )
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete usr_badges error: {}", e),
                        )
                    })?;
                // points (corp_id, from, to)
                points::Entity::delete_many()
                    .filter(
                        Condition::any()
                            .add(points::Column::CorpId.eq(uid))
                            .add(points::Column::From.eq(uid))
                            .add(points::Column::To.eq(uid)),
                    )
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete points error: {}", e),
                        )
                    })?;
                // payments (corp_id)
                payments::Entity::delete_many()
                    .filter(payments::Column::CorpId.eq(uid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete payments error: {}", e),
                        )
                    })?;
                // payouts (usr_id)
                payouts::Entity::delete_many()
                    .filter(payouts::Column::UsrId.eq(uid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete payouts error: {}", e),
                        )
                    })?;
                // jobs (corp_id)
                jobs::Entity::delete_many()
                    .filter(jobs::Column::CorpId.eq(uid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete jobs error: {}", e),
                        )
                    })?;
                // badges (corp_id)
                badges::Entity::delete_many()
                    .filter(badges::Column::CorpId.eq(uid))
                    .exec(tx)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            format!("Delete badges error: {}", e),
                        )
                    })?;
            }
            log::debug!("<UsrBl> delete_usr: Finally deleting user record itself.");
            model.delete(tx).await.map_err(|e| {
                ApiError::new_system(
                    ST_INTERNAL_SERVER_ERROR,
                    rterr::ERR_DATABASE,
                    format!("Delete user error: {}", e),
                )
            })?;
            log::debug!("<UsrBl> delete_usr: Transaction success.");
            Ok(())
        })
    })
    .await?;
    // --------------------------------
    // 3. 最終レスポンス
    // --------------------------------
    Ok(DeleteUsrRes { id: target_usr_id })
}
// ============================================================
// Staff Management Hire
// ============================================================
pub async fn hire_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<HireUsrRes, ApiError> {
    log::debug!("<UsrBl> hire_usr: Fetching target user: {}", target_usr_id);
    // 1. 権限チェックと対象ユーザーの取得 (VDRのパーティション内かつ is_staff=0)
    let model = find_usrs_base(ju, ids)
        .await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .filter(usrs::Column::IsStaff.eq(0))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch user error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                ST_NOT_FOUND,
                rterr::ERR_INVALID_REQUEST,
                "User not found or already a staff.",
            )
        })?;

    log::debug!(
        "<UsrBl> hire_usr: Setting is_staff=1 for {}.",
        target_usr_id
    );

    // 2. 更新
    let mut active = model.into_active_model();
    // is_staff is i8 in DB but might be mapped to bool/i8. If error said expected bool, use bool.
    // If error said expected i8 found u8 (no, it said expected bool found integer).
    // So is_staff IS bool. SeaORM maps TINYINT(1) to bool sometimes.
    active.is_staff = Set(true);
    active.update(conn).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Update user staff status error: {}", e),
        )
    })?;

    Ok(HireUsrRes { id: target_usr_id })
}

// ============================================================
// Staff Management Dehire
// ============================================================
pub async fn dehire_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<DehireUsrRes, ApiError> {
    log::debug!(
        "<UsrBl> dehire_usr: Fetching target user: {}",
        target_usr_id
    );
    // 1. 権限チェックと対象ユーザーの取得 (VDRのパーティション内かつ is_staff=1)
    let model = find_usrs_base(ju, ids)
        .await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .filter(usrs::Column::IsStaff.eq(1))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch user error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                ST_NOT_FOUND,
                rterr::ERR_INVALID_REQUEST,
                "User not found or not a staff.",
            )
        })?;

    log::debug!(
        "<UsrBl> dehire_usr: Setting is_staff=0 for {}.",
        target_usr_id
    );

    // 2. 更新
    let mut active = model.into_active_model();
    active.is_staff = Set(false);
    active.update(conn).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Update user staff status error: {}", e),
        )
    })?;

    Ok(DehireUsrRes { id: target_usr_id })
}

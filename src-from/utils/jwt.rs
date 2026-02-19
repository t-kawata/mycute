use chrono::{Utc, TimeDelta};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, prelude::Expr, ColumnTrait};
use crate::entities::usrs;
use crate::vo::usrs_vo::AuthUsrVo;
use crate::utils::crypto::verify_hash;
use crate::utils::bd::is_valid_bd;
use anyhow::{Result, anyhow};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};
use crate::mode::rt::{rtres::errs_res::ApiError, rterr::rterr};
use std::sync::Arc;

pub struct JwtConfig {
    pub skey: String,
    pub crypto_key: String,
}

/// JWTのペイロード（Claims）構造体
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub apx_id: u32,
    pub vdr_id: u32,
    pub usr_id: u32,
    pub email: String,
    #[serde(rename = "type")]
    pub usr_type: u8,
    pub is_staff: bool,
    pub exp: i64, // Unixタイムスタンプ
}

#[derive(Clone)]
pub struct JwtUsr {
    pub apx_id: u32,
    pub vdr_id: u32,
    pub usr_id: u32,
    pub staff_id: Option<u32>,
    pub email: String,
    pub usr_type: u8,
}

#[derive(Clone)]
pub struct JwtIDs {
    pub apx_id: u32,
    pub vdr_id: u32,
    pub usr_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtRole {
    BD,
    APX,
    VDR,
    USR,
}

impl JwtUsr {
    pub fn is_bd(&self) -> bool {
        is_bd(&self.apx_id, &self.vdr_id, &self.usr_id)
    }
    pub fn is_apx(&self) -> bool {
        is_apx(&self.apx_id, &self.vdr_id, &self.usr_id)
    }
    pub fn is_vdr(&self) -> bool {
        is_vdr(&self.apx_id, &self.vdr_id, &self.usr_id)
    }
    pub fn is_usr(&self) -> bool {
        is_usr(&self.apx_id, &self.vdr_id, &self.usr_id)
    }

    pub fn role(&self) -> JwtRole {
        if self.is_bd() { JwtRole::BD }
        else if self.is_apx() { JwtRole::APX }
        else if self.is_vdr() { JwtRole::VDR }
        else { JwtRole::USR }
    }

    pub fn allow_roles(&self, roles: &[JwtRole]) -> Result<(), ApiError> {
        let current_role = self.role();
        if roles.contains(&current_role) {
            Ok(())
        } else {
            Err(ApiError::new_system(
                ST_FORBIDDEN,
                rterr::ERR_AUTH,
                format!("Access denied for {:?}. Allowed: {:?}", current_role, roles)
            ))
        }
    }

    pub fn ids(&self) -> JwtIDs {
        if self.is_bd() {
            JwtIDs { apx_id: 0, vdr_id: 0, usr_id: 0 }
        } else if self.is_apx() {
            JwtIDs { apx_id: self.usr_id, vdr_id: 0, usr_id: self.usr_id }
        } else if self.is_vdr() {
            JwtIDs { apx_id: self.apx_id, vdr_id: self.usr_id, usr_id: self.usr_id }
        } else {
            // is_usr or others
            JwtIDs { apx_id: self.apx_id, vdr_id: self.vdr_id, usr_id: self.usr_id }
        }
    }
}

impl From<Claims> for JwtUsr {
    fn from(c: Claims) -> Self {
        Self {
            apx_id: c.apx_id,
            vdr_id: c.vdr_id,
            usr_id: c.usr_id,
            staff_id: if c.is_staff { Some(c.usr_id) } else { Some(0) },
            email: c.email,
            usr_type: c.usr_type,
        }
    }
}

impl<S> FromRequestParts<S> for JwtUsr
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // すでに検証済みなら拡張から返す
        if let Some(ju) = parts.extensions.get::<JwtUsr>() {
            return Ok(ju.clone());
        }
        let auth_header = parts.headers.get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| ApiError::new_system(ST_UNAUTHORIZED, rterr::ERR_AUTH, "Missing Authorization header."))?;
        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::new_system(ST_UNAUTHORIZED, rterr::ERR_AUTH, "Invalid Authorization header format."));
        }
        let token = &auth_header[7..];
        let jwt_config = parts.extensions.get::<Arc<JwtConfig>>()
            .ok_or_else(|| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, "JwtConfig not found in extensions."))?;
        let claims = verify_token(&jwt_config.skey, token)
            .map_err(|e| ApiError::new_system(ST_UNAUTHORIZED, rterr::ERR_AUTH, e.to_string()))?;
        let ju = JwtUsr::from(claims);
        let ids = ju.ids();
        let path = parts.uri.path().strip_prefix("/v1").unwrap_or(parts.uri.path());
        let role = if ju.is_bd() { "BD" } else if ju.is_apx() { "APX" } else if ju.is_vdr() { "VDR" } else { "USR" };
        log::debug!("<{} {}> by: {}, apx: {}, vdr: {}, usr: {}", parts.method, path, role, ids.apx_id, ids.vdr_id, ids.usr_id);
        // 拡張に保存して再利用可能にする
        parts.extensions.insert(ju.clone());
        Ok(ju)
    }
}

impl<S> FromRequestParts<S> for JwtIDs
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ju = JwtUsr::from_request_parts(parts, state).await?;
        Ok(ju.ids())
    }
}

pub fn is_bd(aid: &u32, vid: &u32, uid: &u32) -> bool {
    *aid == 0 && *vid == 0 && *uid == 0
}

pub fn is_apx(aid: &u32, vid: &u32, uid: &u32) -> bool {
    *aid == 0 && *vid == 0 && *uid > 0
}

pub fn is_vdr(aid: &u32, vid: &u32, uid: &u32) -> bool {
    *aid > 0 && *vid == 0 && *uid > 0
}

pub fn is_usr(aid: &u32, vid: &u32, uid: &u32) -> bool {
    *aid > 0 && *vid > 0 && *uid > 0
}

pub fn generate_token(
    skey: &str,
    aid: u32,
    vid: u32,
    uid: u32,
    is_staff: bool,
    usr_type: u8,
    email: String,
    expire: u32,
) -> Result<String, jsonwebtoken::errors::Error> {
    let staff_id = if is_staff { Some(uid) } else { Some(0) };
    let jwt_usr = JwtUsr {
        apx_id: aid,
        vdr_id: vid,
        usr_id: uid,
        staff_id,
        email,
        usr_type,
    };
    generate_token_base(skey, expire, &jwt_usr)
}

pub fn generate_token_for_bd(skey: &str, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, 0, 0, 0, false, 0, "bd@bd.com".to_string(), expire)
}

pub fn generate_token_for_apx(skey: &str, uid: u32, email: String, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, 0, 0, uid, false, 0, email, expire)
}

pub fn generate_token_for_vdr(skey: &str, aid: u32, uid: u32, email: String, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, aid, 0, uid, false, 0, email, expire)
}

pub fn generate_token_for_usr(skey: &str, aid: u32, vid: u32, uid: u32, email: String, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, aid, vid, uid, false, 0, email, expire)
}

// ------------------------------------------------------------
// Authentication Logic
// ------------------------------------------------------------

pub async fn auth_bd(conn: &DatabaseConnection, x_bd: &str, skey: &str, expire: u32) -> Result<String> {
    let is_valid = is_valid_bd(conn, x_bd.to_string())
        .await
        .map_err(|e| anyhow!("BD verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid BD."));
    }
    generate_token_for_bd(skey, expire).map_err(|e| anyhow!("Token generation error: {}", e))
}

pub async fn auth_apx(conn: &DatabaseConnection, email: String, password: String, skey: &str, expire: u32) -> Result<String> {
    let result = usrs::Entity::find()
        .select_only()
        .column(usrs::Column::Id)
        .column(usrs::Column::Email)
        .column(usrs::Column::Password)
        .filter(usrs::Column::ApxId.is_null())
        .filter(usrs::Column::VdrId.is_null())
        .filter(usrs::Column::Email.eq(email))
        .filter(Expr::col(usrs::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(usrs::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<AuthUsrVo>()
        .one(conn)
        .await
        .map_err(|e| anyhow!("Failed to fetch APX user: {}", e))?;
    let usr = result.ok_or_else(|| anyhow!("Invalid email or password for APX."))?;
    let is_valid = verify_hash(&password, &usr.password)
        .map_err(|e| anyhow!("Password verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid email or password for APX."));
    }
    generate_token_for_apx(skey, usr.id as u32, usr.email, expire).map_err(|e| anyhow!("APX token generation error: {}", e))
}

pub async fn auth_vdr(conn: &DatabaseConnection, apx_id: u32, email: String, password: String, skey: &str, expire: u32) -> Result<String> {
    let result = usrs::Entity::find()
        .select_only()
        .column(usrs::Column::Id)
        .column(usrs::Column::Email)
        .column(usrs::Column::Password)
        .filter(usrs::Column::ApxId.eq(apx_id))
        .filter(usrs::Column::VdrId.is_null())
        .filter(usrs::Column::Email.eq(email))
        .filter(Expr::col(usrs::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(usrs::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<AuthUsrVo>()
        .one(conn)
        .await
        .map_err(|e| anyhow!("Failed to fetch VDR user: {}", e))?;
    let usr = result.ok_or_else(|| anyhow!("Invalid email or password for VDR."))?;
    let is_valid = verify_hash(&password, &usr.password)
        .map_err(|e| anyhow!("Password verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid email or password for VDR."));
    }
    generate_token_for_vdr(skey, apx_id, usr.id as u32, usr.email, expire).map_err(|e| anyhow!("VDR token generation error: {}", e))
}

pub async fn auth_usr(conn: &DatabaseConnection, apx_id: u32, vdr_id: u32, email: String, password: String, skey: &str, expire: u32) -> Result<String> {
    let result = usrs::Entity::find()
        .select_only()
        .column(usrs::Column::Id)
        .column(usrs::Column::Email)
        .column(usrs::Column::Password)
        .filter(usrs::Column::ApxId.eq(apx_id))
        .filter(usrs::Column::VdrId.eq(vdr_id))
        .filter(usrs::Column::Email.eq(email))
        .filter(Expr::col(usrs::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(usrs::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<AuthUsrVo>()
        .one(conn)
        .await
        .map_err(|e| anyhow!("Failed to fetch USR user: {}", e))?;
    let usr = result.ok_or_else(|| anyhow!("Invalid email or password for USR."))?;
    let is_valid = verify_hash(&password, &usr.password)
        .map_err(|e| anyhow!("Password verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid email or password for USR."));
    }
    generate_token_for_usr(skey, apx_id, vdr_id, usr.id as u32, usr.email, expire).map_err(|e| anyhow!("USR token generation error: {}", e))
}

pub fn verify_token(skey: &str, token: &str) -> Result<Claims> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(skey.as_bytes()),
        &validation,
    ).map_err(|e| anyhow!("Token verification failed: {}", e))?;
    Ok(token_data.claims)
}

fn generate_token_base(
    skey: &str,
    life_time: u32,
    u: &JwtUsr,
) -> Result<String, jsonwebtoken::errors::Error> {
    // 有効期限の計算 (現在時刻 + life_time 時間)
    let exp = Utc::now()
        .checked_add_signed(TimeDelta::hours(life_time as i64))
        .expect("valid timestamp")
        .timestamp();
    let claims = Claims {
        apx_id: u.apx_id,
        vdr_id: u.vdr_id,
        usr_id: u.usr_id,
        email: u.email.clone(),
        usr_type: u.usr_type,
        is_staff: u.staff_id.map(|id| id > 0).unwrap_or(false),
        exp,
    };
    // HS256アルゴリズムでエンコード
    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(skey.as_bytes()))
}


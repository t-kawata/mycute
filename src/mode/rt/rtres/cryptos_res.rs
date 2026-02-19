use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct EncryptRes {
    pub data: String,
}

#[derive(Serialize, ToSchema)]
pub struct DecryptRes {
    pub data: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreateVdrTokenRes {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, ToSchema)]
pub struct GetVdrTokenRes {
    pub key: String,
    pub value: String,
}

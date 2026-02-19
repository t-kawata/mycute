use sea_orm::FromQueryResult;

#[derive(FromQueryResult)]
pub struct AuthUsrVo {
    pub id: i32,
    pub email: String,
    pub password: String,
}
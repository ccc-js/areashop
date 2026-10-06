use sea_orm::DbConn;

#[derive(Clone)]
pub struct AppState {
    pub db: DbConn,
    pub jwt_secret: String,
}

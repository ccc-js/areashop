use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// e.g. sqlite://./areashop.db?mode=rwc  or  postgres://user:pass@localhost:5432/areashop
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://./areashop.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-me".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
        }
    }

    /// "sqlite" | "postgres" — 僅依 URL scheme 判斷，上層不寫任何 DB 專屬 SQL
    pub fn db_driver(&self) -> &'static str {
        if self.database_url.starts_with("postgres") {
            "postgres"
        } else {
            "sqlite"
        }
    }
}

mod auth;
mod booking;
mod config;
mod entities;
mod routes;
mod schema;
mod search;
mod seed;
mod state;

use axum::Router;
use sea_orm::Database;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use crate::{config::AppConfig, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = AppConfig::from_env();
    tracing::info!("db_driver={} port={}", cfg.db_driver(), cfg.port);

    // 同一份程式碼連兩種 DB：URL 決定一切
    // sqlite:  sqlite://./areashop.db?mode=rwc
    // postgres: postgres://user:pass@localhost:5432/areashop
    if cfg.db_driver() == "sqlite" {
        if let Some(path) = cfg.database_url.strip_prefix("sqlite://") {
            let file = path.split('?').next().unwrap_or(path);
            if !file.is_empty() && file != ":memory:" {
                if let Some(parent) = std::path::Path::new(file).parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
            }
        }
    }
    let db = Database::connect(&cfg.database_url).await?;
    schema::setup_schema(&db).await?;
    // SEED=0/false/no → 從空資料庫開始（不塞假資料）；預設塞開發用種子資料
    let seed = std::env::var("SEED")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no" | "empty"))
        .unwrap_or(true);
    if seed {
        seed::seed_if_empty(&db).await?;
    } else {
        tracing::info!("SEED=0, skip seed (empty database)");
    }

    let state = AppState {
        db,
        jwt_secret: cfg.jwt_secret.clone(),
    };
    // AuthUser extractor 直接讀 JWT_SECRET env；把 secret 同步過去
    std::env::set_var("JWT_SECRET", &cfg.jwt_secret);

    let app: Router = Router::new()
        .nest("/api/v1", routes::v1_router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

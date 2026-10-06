use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{entities::area, state::AppState};

pub(crate) async fn list(
    State(s): State<AppState>,
) -> Result<Json<Vec<area::Model>>, (StatusCode, String)> {
    area::Entity::find()
        .all(&s.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Serialize)]
pub(crate) struct CountiesResp {
    counties: Vec<String>,
}

pub(crate) async fn counties(
    State(s): State<AppState>,
) -> Result<Json<CountiesResp>, (StatusCode, String)> {
    let all = area::Entity::find()
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut counties: Vec<String> = all.into_iter().map(|a| a.county).collect();
    counties.sort();
    counties.dedup();
    Ok(Json(CountiesResp { counties }))
}

#[derive(Deserialize, Serialize)]
pub(crate) struct TownshipQ {
    county: String,
}

pub(crate) async fn townships(
    State(s): State<AppState>,
    Query(q): Query<TownshipQ>,
) -> Result<Json<Vec<area::Model>>, (StatusCode, String)> {
    area::Entity::find()
        .filter(area::Column::County.eq(q.county))
        .all(&s.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SearchQ {
    q: String,
}

pub(crate) async fn search(
    State(s): State<AppState>,
    Query(q): Query<SearchQ>,
) -> Result<Json<Vec<area::Model>>, (StatusCode, String)> {
    // 跨 DB 通用：先全取再內存過濾（地區表很小，<5000 筆；v0.3 再做 pg_trgm）
    let all = area::Entity::find()
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let out: Vec<area::Model> = all
        .into_iter()
        .filter(|a| {
            a.county.contains(&q.q)
                || a.township.contains(&q.q)
                || a.village.as_deref().unwrap_or("").contains(&q.q)
        })
        .take(50)
        .collect();
    Ok(Json(out))
}

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{auth::AuthUser, entities::shop, state::AppState};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct CreateShop {
    #[validate(length(min = 1, max = 60))]
    name: String,
    area_id: i32,
    #[serde(default = "default_kind")]
    kind: String, // personal | store
    description: Option<String>,
    address_text: Option<String>,
    phone: Option<String>,
    /// 實體店址（有店面才填）
    address: Option<String>,
    /// 營業時間（有店面才填）
    opening_hours: Option<String>,
}

fn default_kind() -> String {
    "personal".to_string()
}

pub(crate) async fn create(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateShop>,
) -> Result<Json<shop::Model>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if req.kind != "personal" && req.kind != "store" {
        return Err((
            StatusCode::BAD_REQUEST,
            "kind must be personal|store".into(),
        ));
    }
    if req.kind == "store" && req.address.as_deref().unwrap_or("").is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "store kind requires address".into(),
        ));
    }
    let am = shop::ActiveModel {
        owner_id: Set(auth.id),
        area_id: Set(req.area_id),
        name: Set(req.name),
        kind: Set(req.kind),
        description: Set(req.description),
        address_text: Set(req.address_text),
        phone: Set(req.phone),
        address: Set(req.address),
        opening_hours: Set(req.opening_hours),
        status: Set("open".to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    let m = am
        .insert(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(m))
}

pub(crate) async fn list_mine(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<shop::Model>>, (StatusCode, String)> {
    use sea_orm::{ColumnTrait, QueryFilter};
    shop::Entity::find()
        .filter(shop::Column::OwnerId.eq(auth.id))
        .all(&s.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub(crate) async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<shop::Model>, (StatusCode, String)> {
    shop::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))
        .map(Json)
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UpdateShop {
    name: Option<String>,
    description: Option<String>,
    address_text: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    opening_hours: Option<String>,
    status: Option<String>, // open | closed
}

pub(crate) async fn update(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<UpdateShop>,
) -> Result<Json<shop::Model>, (StatusCode, String)> {
    let m = shop::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    if m.owner_id != auth.id && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "not your shop".into()));
    }
    let mut am: shop::ActiveModel = m.into();
    if let Some(v) = req.name {
        am.name = Set(v);
    }
    if let Some(v) = req.description {
        am.description = Set(Some(v));
    }
    if let Some(v) = req.address_text {
        am.address_text = Set(Some(v));
    }
    if let Some(v) = req.phone {
        am.phone = Set(Some(v));
    }
    if let Some(v) = req.address {
        am.address = Set(Some(v));
    }
    if let Some(v) = req.opening_hours {
        am.opening_hours = Set(Some(v));
    }
    if let Some(v) = req.status {
        if v != "open" && v != "closed" {
            return Err((StatusCode::BAD_REQUEST, "bad status".into()));
        }
        am.status = Set(v);
    }
    let m = am
        .update(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(m))
}

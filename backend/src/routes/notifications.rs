use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthUser, entities::notification, state::AppState};

/// 寫一則通知（訂單流程里呼叫；失敗直接報錯，不靜默丟失）
pub(crate) async fn push(
    db: &sea_orm::DbConn,
    user_id: i32,
    kind: &str,
    order_id: i32,
    actor: &str,
) -> Result<(), (StatusCode, String)> {
    notification::ActiveModel {
        user_id: Set(user_id),
        kind: Set(kind.to_string()),
        order_id: Set(order_id),
        actor: Set(actor.to_string()),
        read: Set(false),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map(|_| ())
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub(crate) async fn list(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<notification::Model>>, (StatusCode, String)> {
    use sea_orm::PaginatorTrait;
    notification::Entity::find()
        .filter(notification::Column::UserId.eq(auth.id))
        .order_by_desc(notification::Column::Id)
        .paginate(&s.db, 50)
        .fetch_page(0)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, Serialize)]
pub(crate) struct UnreadCount {
    count: u64,
}

pub(crate) async fn unread_count(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UnreadCount>, (StatusCode, String)> {
    use sea_orm::PaginatorTrait;
    let count = notification::Entity::find()
        .filter(notification::Column::UserId.eq(auth.id))
        .filter(notification::Column::Read.eq(false))
        .paginate(&s.db, 1000)
        .num_items()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(UnreadCount { count }))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ReadReq {
    /// 空＝全部已讀
    #[serde(default)]
    ids: Vec<i32>,
}

pub(crate) async fn mark_read(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ReadReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut sel = notification::Entity::find()
        .filter(notification::Column::UserId.eq(auth.id))
        .filter(notification::Column::Read.eq(false));
    if !req.ids.is_empty() {
        sel = sel.filter(notification::Column::Id.is_in(req.ids));
    }
    let rows = sel
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    for m in rows {
        let mut am: notification::ActiveModel = m.into();
        am.read = Set(true);
        am.update(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

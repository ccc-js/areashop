use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::{create_jwt, hash_password, verify_password, AuthUser},
    entities::user,
    state::AppState,
};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct RegisterReq {
    #[validate(length(min = 6, max = 20))]
    phone: String,
    #[validate(length(min = 6, max = 72))]
    password: String,
    #[validate(length(min = 1, max = 30))]
    nickname: String,
    home_area_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthResp {
    access_token: String,
    user: UserView,
}

#[derive(Debug, Serialize)]
pub(crate) struct UserView {
    id: i32,
    phone: String,
    nickname: String,
    home_area_id: Option<i32>,
    role: String,
}

fn view(m: &user::Model) -> UserView {
    UserView {
        id: m.id,
        phone: m.phone.clone(),
        nickname: m.nickname.clone(),
        home_area_id: m.home_area_id,
        role: m.role.clone(),
    }
}

pub(crate) async fn register(
    State(s): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> Result<Json<AuthResp>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let exists = user::Entity::find()
        .filter(user::Column::Phone.eq(&req.phone))
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if exists.is_some() {
        return Err((StatusCode::CONFLICT, "phone already registered".into()));
    }
    let hash = hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let am = user::ActiveModel {
        phone: Set(req.phone),
        password_hash: Set(hash),
        nickname: Set(req.nickname),
        home_area_id: Set(req.home_area_id),
        role: Set("user".to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    let m = am
        .insert(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let token = create_jwt(m.id, &m.role, &s.jwt_secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(AuthResp {
        access_token: token,
        user: view(&m),
    }))
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct LoginReq {
    phone: String,
    password: String,
}

pub(crate) async fn login(
    State(s): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<AuthResp>, (StatusCode, String)> {
    let m = user::Entity::find()
        .filter(user::Column::Phone.eq(&req.phone))
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "invalid phone or password".to_string(),
        ))?;
    if !verify_password(&req.password, &m.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "invalid phone or password".into()));
    }
    let token = create_jwt(m.id, &m.role, &s.jwt_secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(AuthResp {
        access_token: token,
        user: view(&m),
    }))
}

pub(crate) async fn me(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserView>, (StatusCode, String)> {
    let m = user::Entity::find_by_id(auth.id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?;
    Ok(Json(view(&m)))
}

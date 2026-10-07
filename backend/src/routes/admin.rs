use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{auth::AuthUser, entities::area, state::AppState};

fn require_admin(auth: &AuthUser) -> Result<(), (StatusCode, String)> {
    if auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "admin only".into()));
    }
    Ok(())
}

// ---- 地區 CRUD ----

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct CreateArea {
    #[validate(length(min = 1, max = 30))]
    county: String,
    #[validate(length(min = 1, max = 30))]
    township: String,
    village: Option<String>,
    center_lat: Option<f64>,
    center_lng: Option<f64>,
    radius_km: Option<f64>,
}

pub(crate) async fn create_area(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateArea>,
) -> Result<Json<area::Model>, (StatusCode, String)> {
    require_admin(&auth)?;
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 前後空白一律 trim（" 鳳山" 這種不再發生）
    let county = req.county.trim().to_string();
    let township = req.township.trim().to_string();
    let village = req.village.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    if county.is_empty() || township.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "bad area name".into()));
    }
    // 同縣同鄉鎮不重複（village 不同算不同里，允許多筆；NULL 用 IS NULL 比）
    let mut sel = area::Entity::find()
        .filter(area::Column::County.eq(&county))
        .filter(area::Column::Township.eq(&township));
    sel = match &village {
        Some(v) => sel.filter(area::Column::Village.eq(v.clone())),
        None => sel.filter(area::Column::Village.is_null()),
    };
    let dup = sel
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if dup.is_some() {
        return Err((StatusCode::CONFLICT, "area exists".into()));
    }
    let m = area::ActiveModel {
        county: Set(county),
        township: Set(township),
        village: Set(village),
        center_lat: Set(req.center_lat),
        center_lng: Set(req.center_lng),
        radius_km: Set(req.radius_km),
        ..Default::default()
    }
    .insert(&s.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(m))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UpdateArea {
    county: Option<String>,
    township: Option<String>,
    village: Option<String>,
    center_lat: Option<f64>,
    center_lng: Option<f64>,
    radius_km: Option<f64>,
}

pub(crate) async fn update_area(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<UpdateArea>,
) -> Result<Json<area::Model>, (StatusCode, String)> {
    require_admin(&auth)?;
    let m = area::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "area not found".to_string()))?;
    let mut am: area::ActiveModel = m.into();
    if let Some(v) = req.county {
        let v = v.trim().to_string();
        if v.is_empty() || v.len() > 30 {
            return Err((StatusCode::BAD_REQUEST, "bad county".into()));
        }
        am.county = Set(v);
    }
    if let Some(v) = req.township {
        let v = v.trim().to_string();
        if v.is_empty() || v.len() > 30 {
            return Err((StatusCode::BAD_REQUEST, "bad township".into()));
        }
        am.township = Set(v);
    }
    if let Some(v) = req.village {
        let v = v.trim().to_string();
        am.village = Set(if v.is_empty() { None } else { Some(v) });
    }
    if let Some(v) = req.center_lat {
        am.center_lat = Set(Some(v));
    }
    if let Some(v) = req.center_lng {
        am.center_lng = Set(Some(v));
    }
    if let Some(v) = req.radius_km {
        am.radius_km = Set(Some(v));
    }
    let m = am
        .update(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(m))
}

pub(crate) async fn delete_area(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::entities::{shop, user};
    require_admin(&auth)?;
    let m = area::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "area not found".to_string()))?;
    // 還有人住或有店在用就擋（避免孤兒資料）
    let used_shop = shop::Entity::find()
        .filter(shop::Column::AreaId.eq(id))
        .paginate(&s.db, 1)
        .num_items()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let used_user = user::Entity::find()
        .filter(user::Column::HomeAreaId.eq(id))
        .paginate(&s.db, 1)
        .num_items()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if used_shop + used_user > 0 {
        return Err((StatusCode::BAD_REQUEST, "area in use".into()));
    }
    area::Entity::delete_by_id(m.id)
        .exec(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ---- 一覽（唯讀；停權/下架沿用 PATCH /shops/:id、/items/:id，admin 已放行） ----

#[derive(Debug, Serialize)]
pub(crate) struct ShopRow {
    #[serde(flatten)]
    shop: crate::entities::shop::Model,
    owner_nickname: String,
    owner_phone: String,
}

pub(crate) async fn list_shops(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ShopRow>>, (StatusCode, String)> {
    use crate::entities::{shop, user};
    require_admin(&auth)?;
    let shops = shop::Entity::find()
        .order_by_desc(shop::Column::Id)
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut out = vec![];
    for sh in shops {
        let owner = user::Entity::find_by_id(sh.owner_id)
            .one(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let (owner_nickname, owner_phone) = owner
            .map(|u| (u.nickname, u.phone))
            .unwrap_or(("（已刪號）".to_string(), String::new()));
        out.push(ShopRow {
            shop: sh,
            owner_nickname,
            owner_phone,
        });
    }
    Ok(Json(out))
}

#[derive(Debug, Serialize)]
pub(crate) struct ItemRow {
    #[serde(flatten)]
    item: crate::entities::item::Model,
    shop_name: String,
}

pub(crate) async fn list_items(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ItemRow>>, (StatusCode, String)> {
    use crate::entities::{item, shop};
    require_admin(&auth)?;
    let items = item::Entity::find()
        .order_by_desc(item::Column::Id)
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut out = vec![];
    for it in items {
        let shop_name = shop::Entity::find_by_id(it.shop_id)
            .one(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map(|x| x.name)
            .unwrap_or("（店已刪）".to_string());
        out.push(ItemRow {
            item: it,
            shop_name,
        });
    }
    Ok(Json(out))
}

#[derive(Debug, Serialize)]
pub(crate) struct UserRow {
    id: i32,
    phone: String,
    nickname: String,
    role: String,
    home_area_id: Option<i32>,
}

pub(crate) async fn list_users(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<UserRow>>, (StatusCode, String)> {
    use crate::entities::user;
    require_admin(&auth)?;
    let users = user::Entity::find()
        .order_by_desc(user::Column::Id)
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        users
            .into_iter()
            .map(|u| UserRow {
                id: u.id,
                phone: u.phone,
                nickname: u.nickname,
                role: u.role,
                home_area_id: u.home_area_id,
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub(crate) struct OrderRow {
    #[serde(flatten)]
    order: crate::entities::order::Model,
    buyer_nickname: String,
    shop_name: String,
}

pub(crate) async fn list_orders(
    State(s): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<OrderRow>>, (StatusCode, String)> {
    use crate::entities::{order, shop, user};
    require_admin(&auth)?;
    let orders = order::Entity::find()
        .order_by_desc(order::Column::Id)
        .paginate(&s.db, 100)
        .fetch_page(0)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut out = vec![];
    for o in orders {
        let buyer_nickname = user::Entity::find_by_id(o.buyer_id)
            .one(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map(|u| u.nickname)
            .unwrap_or("（已刪號）".to_string());
        let shop_name = shop::Entity::find_by_id(o.shop_id)
            .one(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map(|x| x.name)
            .unwrap_or("（店已刪）".to_string());
        out.push(OrderRow {
            order: o,
            buyer_nickname,
            shop_name,
        });
    }
    Ok(Json(out))
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{auth::AuthUser, entities::product, state::AppState};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct CreateProduct {
    shop_id: i32,
    #[validate(length(min = 1, max = 120))]
    title: String,
    #[serde(default = "default_cat")]
    category: String,
    #[validate(range(min = 1))]
    price_cents: i32,
    #[validate(range(min = 0))]
    stock: i32,
    #[serde(default = "default_unit")]
    unit: String,
    #[serde(default)]
    pickup_places: serde_json::Value,
    pickup_note: Option<String>,
    #[serde(default)]
    images: serde_json::Value,
}

fn default_cat() -> String {
    "other".to_string()
}
fn default_unit() -> String {
    "份".to_string()
}

fn norm_json(v: &serde_json::Value, fallback_array: bool) -> serde_json::Value {
    if v.is_null() {
        if fallback_array {
            serde_json::json!([])
        } else {
            serde_json::json!([])
        }
    } else {
        v.clone()
    }
}

pub(crate) async fn create(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateProduct>,
) -> Result<Json<product::Model>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 必須是自己的店
    let shop = crate::entities::shop::Entity::find_by_id(req.shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    if shop.owner_id != auth.id && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "not your shop".into()));
    }
    let am = product::ActiveModel {
        shop_id: Set(req.shop_id),
        title: Set(req.title),
        category: Set(req.category),
        price_cents: Set(req.price_cents),
        stock: Set(req.stock),
        unit: Set(req.unit),
        pickup_places: Set(norm_json(&req.pickup_places, true)),
        pickup_note: Set(req.pickup_note),
        images: Set(norm_json(&req.images, true)),
        status: Set("on".to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    let m = am
        .insert(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(m))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ListQ {
    area_id: Option<i32>,
    /// 不分區：全縣，如 county=金門縣（area_id 優先）
    county: Option<String>,
    q: Option<String>,
    category: Option<String>,
    shop_id: Option<i32>,
    #[serde(default = "d_page")]
    page: u64,
    #[serde(default = "d_size")]
    page_size: u64,
}
fn d_page() -> u64 {
    1
}
fn d_size() -> u64 {
    20
}

#[derive(Debug, Serialize)]
pub(crate) struct Page<T: Serialize> {
    items: Vec<T>,
    total: u64,
    page: u64,
    page_size: u64,
}

pub(crate) async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQ>,
) -> Result<Json<Page<product::Model>>, (StatusCode, String)> {
    use crate::entities::shop;
    let page_size = q.page_size.clamp(1, 100);
    let page = q.page.max(1) - 1;

    // 地區過濾：area_id（單鄉鎮）優先，否則 county（不分區=全縣）；
    // 作法：先查出該範圍的 shop_ids 再用 IN，sqlite / postgres 通用。
    let mut sel = product::Entity::find().order_by_desc(product::Column::Id);
    let county_ids: Vec<i32> = if q.area_id.is_none() {
        match &q.county {
            Some(c) if !c.is_empty() => {
                crate::entities::area::Entity::find()
                    .filter(crate::entities::area::Column::County.eq(c))
                    .all(&s.db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    .into_iter()
                    .map(|a| a.id)
                    .collect()
            }
            _ => vec![],
        }
    } else {
        vec![]
    };
    if let Some(area_id) = q.area_id {
        let shops = shop::Entity::find()
            .filter(shop::Column::AreaId.eq(area_id))
            .all(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let ids: Vec<i32> = shops.into_iter().map(|x| x.id).collect();
        if ids.is_empty() {
            return Ok(Json(Page {
                items: vec![],
                total: 0,
                page: page + 1,
                page_size,
            }));
        }
        sel = sel.filter(product::Column::ShopId.is_in(ids));
    }
    if !county_ids.is_empty() || q.area_id.is_none() && q.county.as_deref().is_some_and(|c| !c.is_empty()) {
        // 有指定 county（含該縣無任何地區/店家）→ 限定在該縣範圍內，
        // 避免無範圍時回傳全站商品
        let ids: Vec<i32> = if county_ids.is_empty() {
            vec![-1]
        } else {
            shop::Entity::find()
                .filter(shop::Column::AreaId.is_in(county_ids))
                .all(&s.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .into_iter()
                .map(|x| x.id)
                .collect()
        };
        if ids.is_empty() {
            return Ok(Json(Page {
                items: vec![],
                total: 0,
                page: page + 1,
                page_size,
            }));
        }
        sel = sel.filter(product::Column::ShopId.is_in(ids));
    }
    if let Some(shop_id) = q.shop_id {
        sel = sel.filter(product::Column::ShopId.eq(shop_id));
    }
    if let Some(cat) = q.category {
        if !cat.is_empty() {
            sel = sel.filter(product::Column::Category.eq(cat));
        }
    }
    // q 關鍵字：跨 DB 通用內存過濾會全表掃；MVP 資料小可接受（v0.3 上 pg_trgm）
    let mut all = sel
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if let Some(kw) = q.q {
        if !kw.is_empty() {
            all = all.into_iter().filter(|p| p.title.contains(&kw)).collect();
        }
    }
    // 只看上架
    all = all.into_iter().filter(|p| p.status == "on").collect();
    let total = all.len() as u64;
    let start = (page * page_size) as usize;
    let items: Vec<product::Model> = all
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();
    Ok(Json(Page {
        items,
        total,
        page: page + 1,
        page_size,
    }))
}

pub(crate) async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<product::Model>, (StatusCode, String)> {
    product::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "product not found".to_string()))
        .map(Json)
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UpdateProduct {
    title: Option<String>,
    price_cents: Option<i32>,
    stock: Option<i32>,
    status: Option<String>,
    pickup_note: Option<String>,
}

pub(crate) async fn update(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProduct>,
) -> Result<Json<product::Model>, (StatusCode, String)> {
    let m = product::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "product not found".to_string()))?;
    let shop = crate::entities::shop::Entity::find_by_id(m.shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    if shop.owner_id != auth.id && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "not your product".into()));
    }
    let mut am: product::ActiveModel = m.into();
    if let Some(v) = req.title {
        am.title = Set(v);
    }
    if let Some(v) = req.price_cents {
        if v <= 0 {
            return Err((StatusCode::BAD_REQUEST, "bad price".into()));
        }
        am.price_cents = Set(v);
    }
    if let Some(v) = req.stock {
        if v < 0 {
            return Err((StatusCode::BAD_REQUEST, "bad stock".into()));
        }
        am.stock = Set(v);
    }
    if let Some(v) = req.status {
        if !["on", "off", "soldout"].contains(&v.as_str()) {
            return Err((StatusCode::BAD_REQUEST, "bad status".into()));
        }
        am.status = Set(v);
    }
    if let Some(v) = req.pickup_note {
        am.pickup_note = Set(Some(v));
    }
    let m = am
        .update(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(m))
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::AuthUser,
    booking::{self, ExcView, RuleView, Window},
    entities::{availability_exception, availability_rule, item},
    state::AppState,
};

fn d_page() -> u64 {
    1
}
fn d_size() -> u64 {
    20
}
fn d_cancel() -> i32 {
    24
}

// 同一模式上架：賣東西（stock 有值）和賣服務（bookable＋時間）可並存，至少要有一種
#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct CreateItem {
    shop_id: i32,
    #[validate(length(min = 1, max = 120))]
    title: String,
    description: Option<String>,
    #[validate(range(min = 0))]
    price_cents: i32,
    #[serde(default = "default_cat")]
    category: String,
    #[serde(default = "default_unit")]
    unit: String,
    #[serde(default)]
    images: serde_json::Value,
    /// 庫存；None = 不限量（服務型）。至少 stock/bookable 要有一個。
    stock: Option<i32>,
    #[serde(default)]
    bookable: bool,
    #[serde(default = "d_cancel")]
    cancel_hours: i32,
    notice: Option<String>,
}

fn default_unit() -> String {
    "份".to_string()
}

fn default_cat() -> String {
    "other".to_string()
}

async fn require_owner(
    s: &AppState,
    auth: &AuthUser,
    shop_id: i32,
) -> Result<(), (StatusCode, String)> {
    let shop = crate::entities::shop::Entity::find_by_id(shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    if shop.owner_id != auth.id && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "not your shop".into()));
    }
    Ok(())
}

pub(crate) async fn create(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateItem>,
) -> Result<Json<item::Model>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if !crate::search::valid_category(&req.category) {
        return Err((StatusCode::BAD_REQUEST, "bad category".into()));
    }
    if let Some(st) = req.stock {
        if st < 0 {
            return Err((StatusCode::BAD_REQUEST, "bad stock".into()));
        }
    }
    if req.stock.is_none() && !req.bookable {
        return Err((StatusCode::BAD_REQUEST, "need stock or bookable".into()));
    }
    if req.cancel_hours < 0 {
        return Err((StatusCode::BAD_REQUEST, "bad cancel_hours".into()));
    }
    require_owner(&s, &auth, req.shop_id).await?;
    let m = item::ActiveModel {
        shop_id: Set(req.shop_id),
        title: Set(req.title),
        description: Set(req.description),
        price_cents: Set(req.price_cents),
        category: Set(req.category),
        unit: Set(req.unit),
        images: Set(if req.images.is_null() {
            serde_json::json!([])
        } else {
            req.images
        }),
        status: Set("on".to_string()),
        stock: Set(req.stock),
        bookable: Set(req.bookable),
        cancel_hours: Set(req.cancel_hours),
        notice: Set(req.notice),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
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
    shop_id: Option<i32>,
    /// 分類過濾：fresh | food | daily | service | other
    category: Option<String>,
    /// 只看可預約
    bookable: Option<bool>,
    #[serde(default = "d_page")]
    page: u64,
    #[serde(default = "d_size")]
    page_size: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct Page<T: Serialize> {
    items: Vec<T>,
    total: u64,
    page: u64,
    page_size: u64,
}

fn empty_page(page: u64, page_size: u64) -> Page<item::Model> {
    Page {
        items: vec![],
        total: 0,
        page: page + 1,
        page_size,
    }
}

pub(crate) async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQ>,
) -> Result<Json<Page<item::Model>>, (StatusCode, String)> {
    use crate::entities::shop;
    let page_size = q.page_size.clamp(1, 100);
    let page = q.page.max(1) - 1;

    // 地區過濾：area_id（單鄉鎮）優先，否則 county（不分區=全縣）；
    // 作法：先查出該範圍的 shop_ids 再用 IN，sqlite / postgres 通用。
    let mut sel = item::Entity::find().order_by_desc(item::Column::Id);
    let county_ids: Vec<i32> = if q.area_id.is_none() {
        match &q.county {
            Some(c) if !c.is_empty() => crate::entities::area::Entity::find()
                .filter(crate::entities::area::Column::County.eq(c))
                .all(&s.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .into_iter()
                .map(|a| a.id)
                .collect(),
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
            return Ok(Json(empty_page(page, page_size)));
        }
        sel = sel.filter(item::Column::ShopId.is_in(ids));
    }
    if !county_ids.is_empty()
        || q.area_id.is_none() && q.county.as_deref().is_some_and(|c| !c.is_empty())
    {
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
            return Ok(Json(empty_page(page, page_size)));
        }
        sel = sel.filter(item::Column::ShopId.is_in(ids));
    }
    if let Some(shop_id) = q.shop_id {
        sel = sel.filter(item::Column::ShopId.eq(shop_id));
    }
    if q.bookable == Some(true) {
        sel = sel.filter(item::Column::Bookable.eq(true));
    }
    if let Some(cat) = &q.category {
        if !cat.is_empty() {
            if !crate::search::valid_category(cat) {
                return Err((StatusCode::BAD_REQUEST, "bad category".into()));
            }
            sel = sel.filter(item::Column::Category.eq(cat.clone()));
        }
    }
    // q 關鍵字：中文分詞簡易版（標題＋描述，AND），跨 DB 通用內存過濾；
    // MVP 資料小可接受（v0.4 上 pg_trgm）
    let mut all = sel
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if let Some(kw) = q.q {
        if !kw.is_empty() {
            let tokens = crate::search::tokenize(&kw);
            all.retain(|p| {
                crate::search::matches(
                    &format!("{} {}", p.title, p.description.as_deref().unwrap_or("")),
                    &tokens,
                )
            });
        }
    }
    // 只看上架
    all.retain(|p| p.status == "on");
    let total = all.len() as u64;
    let start = (page * page_size) as usize;
    let items: Vec<item::Model> = all
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

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct GetQ {
    month: Option<String>, // YYYY-MM，預設本月（UTC）；只有 bookable 才有 days
}

#[derive(Debug, Serialize)]
pub(crate) struct DayStatus {
    date: String,
    open: bool,
    /// 店家手動勾的當日額滿
    full: bool,
    windows: Vec<Window>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ItemDetail {
    #[serde(flatten)]
    item: item::Model,
    rules: Vec<availability_rule::Model>,
    days: Vec<DayStatus>,
}

pub(crate) async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<i32>,
    Query(q): Query<GetQ>,
) -> Result<Json<ItemDetail>, (StatusCode, String)> {
    let m = item::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "item not found".to_string()))?;
    if !m.bookable {
        return Ok(Json(ItemDetail {
            item: m,
            rules: vec![],
            days: vec![],
        }));
    }
    let rule_rows = availability_rule::Entity::find()
        .filter(availability_rule::Column::ItemId.eq(id))
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let month = q
        .month
        .unwrap_or_else(|| Utc::now().format("%Y-%m").to_string());
    let (y, mo) = booking::parse_month(&month)
        .ok_or((StatusCode::BAD_REQUEST, "bad month, use YYYY-MM".into()))?;
    let dates = booking::month_dates(y, mo);
    let exc_rows = availability_exception::Entity::find()
        .filter(availability_exception::Column::ItemId.eq(id))
        .filter(availability_exception::Column::Date.gte(dates.first().cloned().unwrap()))
        .filter(availability_exception::Column::Date.lte(dates.last().cloned().unwrap()))
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let rules: Vec<RuleView> = rule_rows
        .iter()
        .map(|r| RuleView {
            weekday: r.weekday,
            open: r.open,
            windows: booking::parse_windows(&r.windows),
        })
        .collect();
    let excs: Vec<ExcView> = exc_rows
        .iter()
        .map(|e| ExcView {
            date: e.date.clone(),
            open: e.open,
            full: e.full,
        })
        .collect();
    let days = dates
        .into_iter()
        .map(|date| {
            let plan = booking::resolve(&rules, &excs, &date).unwrap();
            DayStatus {
                date,
                open: plan.open,
                full: plan.full,
                windows: plan.windows,
            }
        })
        .collect();
    Ok(Json(ItemDetail {
        item: m,
        rules: rule_rows,
        days,
    }))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UpdateItem {
    title: Option<String>,
    description: Option<String>,
    price_cents: Option<i32>,
    category: Option<String>,
    unit: Option<String>,
    /// 庫存；配合 unlimited 使用
    stock: Option<i32>,
    /// true = 改為不限量（stock 清掉）
    unlimited: Option<bool>,
    bookable: Option<bool>,
    cancel_hours: Option<i32>,
    notice: Option<String>,
    status: Option<String>, // on | off
}

pub(crate) async fn update(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<UpdateItem>,
) -> Result<Json<item::Model>, (StatusCode, String)> {
    let m = item::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "item not found".to_string()))?;
    require_owner(&s, &auth, m.shop_id).await?;
    let mut am: item::ActiveModel = m.into();
    if let Some(v) = req.title {
        if v.is_empty() || v.len() > 120 {
            return Err((StatusCode::BAD_REQUEST, "bad title".into()));
        }
        am.title = Set(v);
    }
    if let Some(v) = req.description {
        am.description = Set(Some(v));
    }
    if let Some(v) = req.price_cents {
        if v < 0 {
            return Err((StatusCode::BAD_REQUEST, "bad price".into()));
        }
        am.price_cents = Set(v);
    }
    if let Some(v) = req.category {
        if !crate::search::valid_category(&v) {
            return Err((StatusCode::BAD_REQUEST, "bad category".into()));
        }
        am.category = Set(v);
    }
    if let Some(v) = req.unit {
        am.unit = Set(v);
    }
    if req.unlimited == Some(true) {
        am.stock = Set(None);
    } else if let Some(v) = req.stock {
        if v < 0 {
            return Err((StatusCode::BAD_REQUEST, "bad stock".into()));
        }
        am.stock = Set(Some(v));
    }
    if let Some(v) = req.bookable {
        am.bookable = Set(v);
    }
    if let Some(v) = req.cancel_hours {
        if v < 0 {
            return Err((StatusCode::BAD_REQUEST, "bad cancel_hours".into()));
        }
        am.cancel_hours = Set(v);
    }
    if let Some(v) = req.notice {
        am.notice = Set(Some(v));
    }
    if let Some(v) = req.status {
        if !["on", "off", "soldout"].contains(&v.as_str()) {
            return Err((StatusCode::BAD_REQUEST, "bad status".into()));
        }
        am.status = Set(v);
    }
    let m = am
        .update(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // 至少留一種賣法
    if m.stock.is_none() && !m.bookable {
        return Err((StatusCode::BAD_REQUEST, "need stock or bookable".into()));
    }
    Ok(Json(m))
}

// ---- 週範本：整包覆寫 ----

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct WeekRule {
    weekday: i32, // 0=週日..6=週六
    open: bool,
    #[serde(default)]
    windows: Vec<Window>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PutRules {
    weekly: Vec<WeekRule>,
}

pub(crate) async fn put_rules(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<PutRules>,
) -> Result<Json<Vec<availability_rule::Model>>, (StatusCode, String)> {
    let m = item::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "item not found".to_string()))?;
    require_owner(&s, &auth, m.shop_id).await?;
    if req.weekly.len() > 7 {
        return Err((StatusCode::BAD_REQUEST, "at most 7 days".into()));
    }
    let mut seen = std::collections::HashSet::new();
    for w in &req.weekly {
        if !(0..=6).contains(&w.weekday) || !seen.insert(w.weekday) {
            return Err((StatusCode::BAD_REQUEST, "bad weekday".into()));
        }
        if w.windows.len() > 8 {
            return Err((StatusCode::BAD_REQUEST, "too many windows".into()));
        }
        for win in &w.windows {
            if !booking::valid_window(&win.start, &win.end) {
                return Err((StatusCode::BAD_REQUEST, "bad window".into()));
            }
        }
    }
    let out =
        s.db.transaction::<_, Vec<availability_rule::Model>, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                availability_rule::Entity::delete_many()
                    .filter(availability_rule::Column::ItemId.eq(id))
                    .exec(txn)
                    .await?;
                let mut rows = vec![];
                for w in &req.weekly {
                    let am = availability_rule::ActiveModel {
                        item_id: Set(id),
                        weekday: Set(w.weekday),
                        open: Set(w.open),
                        windows: Set(serde_json::to_value(&w.windows).unwrap()),
                        created_at: Set(Utc::now()),
                        ..Default::default()
                    };
                    rows.push(am.insert(txn).await?);
                }
                Ok(rows)
            })
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(out))
}

// ---- 單日例外：upsert / 刪除回範本 ----

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct UpsertException {
    #[validate(length(equal = 10))]
    date: String, // YYYY-MM-DD
    open: bool,
    /// 當日額滿（店家手動勾）
    #[serde(default)]
    full: bool,
    note: Option<String>,
}

pub(crate) async fn post_exception(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<UpsertException>,
) -> Result<Json<availability_exception::Model>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let m = item::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "item not found".to_string()))?;
    require_owner(&s, &auth, m.shop_id).await?;
    if booking::parse_date(&req.date).is_none() {
        return Err((StatusCode::BAD_REQUEST, "bad date, use YYYY-MM-DD".into()));
    }
    let exists = availability_exception::Entity::find()
        .filter(availability_exception::Column::ItemId.eq(id))
        .filter(availability_exception::Column::Date.eq(&req.date))
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let out = match exists {
        Some(row) => {
            let mut am: availability_exception::ActiveModel = row.into();
            am.open = Set(req.open);
            am.full = Set(req.full);
            am.note = Set(req.note);
            am.update(&s.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        }
        None => availability_exception::ActiveModel {
            item_id: Set(id),
            date: Set(req.date),
            open: Set(req.open),
            full: Set(req.full),
            note: Set(req.note),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
    };
    Ok(Json(out))
}

pub(crate) async fn delete_exception(
    State(s): State<AppState>,
    auth: AuthUser,
    Path((id, date)): Path<(i32, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let m = item::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "item not found".to_string()))?;
    require_owner(&s, &auth, m.shop_id).await?;
    availability_exception::Entity::delete_many()
        .filter(availability_exception::Column::ItemId.eq(id))
        .filter(availability_exception::Column::Date.eq(&date))
        .exec(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

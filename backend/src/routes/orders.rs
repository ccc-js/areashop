use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::AuthUser,
    booking::{self, ExcView, RuleView},
    entities::{
        item,
        order::{next_status, OrderAction},
        order_item,
    },
    state::AppState,
};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct CreateOrder {
    #[validate(length(min = 1))]
    items: Vec<OrderLine>,
    pickup_at: Option<String>,
    /// 預約日期 YYYY-MM-DD；None = 直接買
    date: Option<String>,
    /// 區段文字（有日期時，買家從當日 windows 選一個，可空）
    window: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct OrderLine {
    item_id: i32,
    #[validate(range(min = 1))]
    qty: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct BuyerView {
    nickname: String,
    phone: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ItemView {
    #[serde(flatten)]
    item: order_item::Model,
    title: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct OrderDetail {
    #[serde(flatten)]
    order: crate::entities::order::Model,
    buyer: BuyerView,
    items: Vec<ItemView>,
}

async fn detail(
    db: &sea_orm::DbConn,
    order: crate::entities::order::Model,
) -> Result<OrderDetail, (StatusCode, String)> {
    use crate::entities::user;
    let buyer = user::Entity::find_by_id(order.buyer_id)
        .one(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "buyer not found".to_string()))?;
    let items = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(order.id))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut views = vec![];
    for it in items {
        // 項目被刪就顯示「（已下架）」，單照留
        let title = item::Entity::find_by_id(it.item_id)
            .one(db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map(|p| p.title)
            .unwrap_or("（已下架）".to_string());
        views.push(ItemView { item: it, title });
    }
    Ok(OrderDetail {
        order,
        buyer: BuyerView {
            nickname: buyer.nickname,
            phone: buyer.phone,
        },
        items: views,
    })
}

/// 某項目某日是否可約（例外 > 週範本）。不可預約的項目直接擋掉。
async fn check_bookable(
    db: &impl ConnectionTrait,
    it: &item::Model,
    date: &str,
    window: &Option<String>,
) -> Result<(), (StatusCode, String)> {
    use crate::entities::{availability_exception, availability_rule};
    if !it.bookable {
        return Err((StatusCode::BAD_REQUEST, "item not bookable".into()));
    }
    let rule_rows = availability_rule::Entity::find()
        .filter(availability_rule::Column::ItemId.eq(it.id))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let exc_rows = availability_exception::Entity::find()
        .filter(availability_exception::Column::ItemId.eq(it.id))
        .filter(availability_exception::Column::Date.eq(date))
        .all(db)
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
    let plan = booking::resolve(&rules, &excs, date)
        .ok_or((StatusCode::BAD_REQUEST, "bad date".into()))?;
    if !plan.open {
        // 不營業或店家手動勾了額滿都擋在這裡（人數不進系統）
        return Err((StatusCode::BAD_REQUEST, "closed on this date".into()));
    }
    if let Some(w) = window {
        if !w.is_empty()
            && !plan.windows.is_empty()
            && !plan.windows.iter().any(|x| x.label() == *w)
        {
            return Err((StatusCode::BAD_REQUEST, "bad window".into()));
        }
    }
    Ok(())
}

fn active_statuses() -> Vec<String> {
    vec!["pending".to_string(), "confirmed".to_string()]
}

pub(crate) async fn create(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOrder>,
) -> Result<Json<OrderDetail>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 有日期 = 預約：檢查格式＋不可約過去
    if let Some(d) = &req.date {
        let day = booking::parse_date(d)
            .ok_or((StatusCode::BAD_REQUEST, "bad date, use YYYY-MM-DD".into()))?;
        if day < Utc::now().date_naive() {
            return Err((StatusCode::BAD_REQUEST, "cannot book past date".into()));
        }
    }
    // 整單必須同一家店（面交簡化；跨店請分開下單）
    let order_id =
        s.db.transaction::<_, i32, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let mut shop_id: Option<i32> = None;
                let mut total = 0i32;
                let mut lines: Vec<(item::Model, i32)> = vec![];
                for line in &req.items {
                    let p = item::Entity::find_by_id(line.item_id)
                        .one(txn)
                        .await?
                        .ok_or(sea_orm::DbErr::RecordNotFound(format!(
                            "item {} not found",
                            line.item_id
                        )))?;
                    if p.status != "on" {
                        return Err(sea_orm::DbErr::Custom("item not on sale".into()));
                    }
                    // 有庫存才檢查（None = 不限量）
                    if let Some(st) = p.stock {
                        if st < line.qty {
                            return Err(sea_orm::DbErr::Custom(format!(
                                "stock not enough for {}",
                                p.title
                            )));
                        }
                    }
                    match shop_id {
                        None => shop_id = Some(p.shop_id),
                        Some(id) if id != p.shop_id => {
                            return Err(sea_orm::DbErr::Custom(
                                "multi-shop order not supported".into(),
                            ))
                        }
                        _ => {}
                    }
                    total += p.price_cents * line.qty;
                    lines.push((p, line.qty));
                }
                let shop_id = shop_id.ok_or(sea_orm::DbErr::Custom("empty order".into()))?;
                // 有日期：每項都要可約＋防同人同項目同日重複送單
                if let Some(date) = &req.date {
                    for (p, _) in &lines {
                        check_bookable(txn, p, date, &req.window)
                            .await
                            .map_err(|(_, msg)| sea_orm::DbErr::Custom(msg))?;
                        // 該買家當日在此店已有有效單＋含同項目 → 擋
                        let exists = crate::entities::order::Entity::find()
                            .filter(crate::entities::order::Column::BuyerId.eq(auth.id))
                            .filter(crate::entities::order::Column::ShopId.eq(shop_id))
                            .filter(crate::entities::order::Column::Date.eq(date.clone()))
                            .filter(crate::entities::order::Column::Status.is_in(active_statuses()))
                            .all(txn)
                            .await?;
                        for o in exists {
                            let its = order_item::Entity::find()
                                .filter(order_item::Column::OrderId.eq(o.id))
                                .filter(order_item::Column::ItemId.eq(p.id))
                                .one(txn)
                                .await?;
                            if its.is_some() {
                                return Err(sea_orm::DbErr::Custom("already booked".into()));
                            }
                        }
                    }
                }
                // 扣庫存（有庫存的才扣；sqlite/postgres 通用，不用 SELECT FOR UPDATE）
                for (p, qty) in &lines {
                    if let Some(st) = p.stock {
                        let mut am: item::ActiveModel = item::Entity::find_by_id(p.id)
                            .one(txn)
                            .await?
                            .ok_or(sea_orm::DbErr::RecordNotFound("item gone".into()))?
                            .into();
                        let left = st - *qty;
                        if left < 0 {
                            return Err(sea_orm::DbErr::Custom("stock changed, retry".into()));
                        }
                        am.stock = Set(Some(left));
                        if left == 0 {
                            am.status = Set("soldout".to_string());
                        }
                        am.update(txn).await?;
                    }
                }
                let om = crate::entities::order::ActiveModel {
                    buyer_id: Set(auth.id),
                    shop_id: Set(shop_id),
                    status: Set("pending".to_string()),
                    pickup_at: Set(req.pickup_at.clone()),
                    date: Set(req.date.clone()),
                    window: Set(req.window.clone()),
                    total_cents: Set(total),
                    remark: Set(req.remark.clone()),
                    created_at: Set(Utc::now()),
                    ..Default::default()
                };
                let order = om.insert(txn).await?;
                for (p, qty) in &lines {
                    let im = order_item::ActiveModel {
                        order_id: Set(order.id),
                        item_id: Set(p.id),
                        qty: Set(*qty),
                        price_cents: Set(p.price_cents),
                        ..Default::default()
                    };
                    im.insert(txn).await?;
                }
                // 明細（含買家＋品名）在 txn 外重查，避免把 txn 拉太長
                Ok(order.id)
            })
        })
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let order = crate::entities::order::Entity::find_by_id(order_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap();
    // 通知店主：有人下單了（actor = 買家暱稱）
    let shop = crate::entities::shop::Entity::find_by_id(order.shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    let buyer = crate::entities::user::Entity::find_by_id(order.buyer_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "buyer not found".to_string()))?;
    crate::routes::notifications::push(&s.db, shop.owner_id, "order_created", order.id, &buyer.nickname).await?;
    Ok(Json(detail(&s.db, order).await?))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ListQ {
    role: Option<String>, // buyer | seller
    from: Option<String>, // YYYY-MM-DD（含）
    to: Option<String>,
}

pub(crate) async fn list(
    State(s): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQ>,
) -> Result<Json<Vec<crate::entities::order::Model>>, (StatusCode, String)> {
    use crate::entities::{order, shop};
    let mut sel = order::Entity::find().order_by_desc(order::Column::Id);
    match q.role.as_deref() {
        Some("seller") => {
            let shops = shop::Entity::find()
                .filter(shop::Column::OwnerId.eq(auth.id))
                .all(&s.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let ids: Vec<i32> = shops.into_iter().map(|x| x.id).collect();
            if ids.is_empty() {
                return Ok(Json(vec![]));
            }
            sel = sel.filter(order::Column::ShopId.is_in(ids));
        }
        _ => {
            sel = sel.filter(order::Column::BuyerId.eq(auth.id));
        }
    }
    // 日期篩選只命中「有日期的單」（NULL 比大小恆為假，自然排除）
    if let Some(from) = q.from {
        sel = sel.filter(order::Column::Date.gte(from));
    }
    if let Some(to) = q.to {
        sel = sel.filter(order::Column::Date.lte(to));
    }
    sel.all(&s.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub(crate) async fn get_one(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<OrderDetail>, (StatusCode, String)> {
    let order = crate::entities::order::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "order not found".to_string()))?;
    check_visible(&s, &auth, &order).await?;
    Ok(Json(detail(&s.db, order).await?))
}

pub(crate) async fn check_visible(
    s: &AppState,
    auth: &AuthUser,
    order: &crate::entities::order::Model,
) -> Result<(), (StatusCode, String)> {
    if order.buyer_id == auth.id {
        return Ok(());
    }
    let shop = crate::entities::shop::Entity::find_by_id(order.shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    if shop.owner_id == auth.id || auth.role == "admin" {
        return Ok(());
    }
    Err((StatusCode::FORBIDDEN, "not your order".into()))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TransitionReq {
    action: String, // confirm | ready | complete | cancel | noshow
}

pub(crate) async fn transition(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<crate::entities::order::Model>, (StatusCode, String)> {
    let action = match req.action.as_str() {
        "confirm" => OrderAction::Confirm,
        "ready" => OrderAction::Ready,
        "complete" => OrderAction::Complete,
        "cancel" => OrderAction::Cancel,
        "noshow" => OrderAction::Noshow,
        _ => return Err((StatusCode::BAD_REQUEST, "bad action".into())),
    };
    let order = crate::entities::order::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "order not found".to_string()))?;
    let shop = crate::entities::shop::Entity::find_by_id(order.shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    let is_buyer = order.buyer_id == auth.id;
    let is_seller = shop.owner_id == auth.id;

    // 權限：confirm/ready/noshow=店主；complete=雙方；cancel=雙方（有日期另有取消期限）
    match action {
        OrderAction::Confirm | OrderAction::Ready | OrderAction::Noshow if !is_seller => {
            return Err((StatusCode::FORBIDDEN, "seller only".into()))
        }
        OrderAction::Complete if !(is_buyer || is_seller) => {
            return Err((StatusCode::FORBIDDEN, "not your order".into()))
        }
        OrderAction::Cancel if !(is_buyer || is_seller) => {
            return Err((StatusCode::FORBIDDEN, "not your order".into()))
        }
        _ => {}
    }
    // 取消期限：有日期的單，當日 00:00（UTC）前 cancel_hours 小時內不可取消
    if action == OrderAction::Cancel {
        if let Some(d) = &order.date {
            let svc_hours = cancel_hours_for(&s, &order).await?;
            let day =
                booking::parse_date(d).ok_or((StatusCode::BAD_REQUEST, "bad order date".into()))?;
            let cutoff = day.and_hms_opt(0, 0, 0).unwrap().and_utc()
                - chrono::Duration::hours(svc_hours as i64);
            if Utc::now() >= cutoff {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "too late to cancel".into(),
                ));
            }
        }
    }
    let next = next_status(&order.status, action)
        .ok_or((StatusCode::BAD_REQUEST, "illegal transition".into()))?;
    // 取消要回補庫存（有庫存的才補；事務）
    if next == "cancelled" {
        s.db.transaction::<_, (), sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let items = order_item::Entity::find()
                    .filter(order_item::Column::OrderId.eq(order.id))
                    .all(txn)
                    .await?;
                for it in items {
                    if let Some(p) = item::Entity::find_by_id(it.item_id).one(txn).await? {
                        if p.stock.is_none() {
                            continue;
                        }
                        // stock 回補：用當前值 + qty（事務內讀寫，MVP 足夠）
                        let cur = item::Entity::find_by_id(it.item_id)
                            .one(txn)
                            .await?
                            .unwrap();
                        let mut am: item::ActiveModel = p.into();
                        am.stock = Set(cur.stock.map(|v| v + it.qty));
                        if cur.status == "soldout" {
                            am.status = Set("on".to_string());
                        }
                        am.update(txn).await?;
                    }
                }
                let mut om: crate::entities::order::ActiveModel =
                    crate::entities::order::Entity::find_by_id(id)
                        .one(txn)
                        .await?
                        .ok_or(sea_orm::DbErr::RecordNotFound("order gone".into()))?
                        .into();
                om.status = Set(next.to_string());
                om.update(txn).await?;
                Ok(())
            })
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        let mut am: crate::entities::order::ActiveModel = order.into();
        am.status = Set(next.to_string());
        am.update(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    let m = crate::entities::order::Entity::find_by_id(id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap();
    // 通知對方：店主動的 → 買家；買家按取消 → 店主（actor = 操作人暱稱）
    let me = crate::entities::user::Entity::find_by_id(auth.id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?;
    let target = if is_seller && !is_buyer {
        m.buyer_id
    } else if is_buyer && !is_seller {
        shop.owner_id
    } else {
        // 既是買家又是店主（自己買自己）：只通知買家身份
        m.buyer_id
    };
    let kind = match action {
        OrderAction::Confirm => "confirmed",
        OrderAction::Ready => "ready",
        OrderAction::Complete => "completed",
        OrderAction::Cancel => "cancelled",
        OrderAction::Noshow => "noshow",
    };
    // 自己通知自己就跳過（自己買自己店的邊緣 case）
    if target != auth.id {
        crate::routes::notifications::push(&s.db, target, kind, m.id, &me.nickname).await?;
    }
    Ok(Json(m))
}

/// 有日期的單，取單內第一個可預約項目的 cancel_hours（整單同店，直接取最大值最保險）
async fn cancel_hours_for(
    s: &AppState,
    order: &crate::entities::order::Model,
) -> Result<i32, (StatusCode, String)> {
    let items = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(order.id))
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut hours = 24;
    for it in items {
        if let Some(p) = item::Entity::find_by_id(it.item_id)
            .one(&s.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            hours = hours.max(p.cancel_hours);
        }
    }
    Ok(hours)
}

// ---- 店家月曆：該店「有日期的單」在某月，按日聚合 ----

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CalendarQ {
    shop_id: i32,
    month: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CalendarDay {
    date: String,
    booked: u64,
    orders: Vec<crate::entities::order::Model>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CalendarResp {
    month: String,
    days: Vec<CalendarDay>,
}

pub(crate) async fn calendar(
    State(s): State<AppState>,
    auth: AuthUser,
    Query(q): Query<CalendarQ>,
) -> Result<Json<CalendarResp>, (StatusCode, String)> {
    use crate::entities::order;
    let shop = crate::entities::shop::Entity::find_by_id(q.shop_id)
        .one(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "shop not found".to_string()))?;
    if shop.owner_id != auth.id && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "not your shop".into()));
    }
    let month = q
        .month
        .unwrap_or_else(|| Utc::now().format("%Y-%m").to_string());
    let (y, mo) = booking::parse_month(&month)
        .ok_or((StatusCode::BAD_REQUEST, "bad month, use YYYY-MM".into()))?;
    let dates = booking::month_dates(y, mo);
    let rows = order::Entity::find()
        .filter(order::Column::ShopId.eq(q.shop_id))
        .filter(order::Column::Date.gte(dates.first().cloned().unwrap()))
        .filter(order::Column::Date.lte(dates.last().cloned().unwrap()))
        .order_by_asc(order::Column::Id)
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut by_date: std::collections::HashMap<String, Vec<order::Model>> =
        std::collections::HashMap::new();
    for r in rows {
        // NULL 日期不會進來（比大小恆為假），這裡都是有日期的單
        by_date
            .entry(r.date.clone().unwrap_or_default())
            .or_default()
            .push(r);
    }
    let days = dates
        .into_iter()
        .map(|date| {
            let items = by_date.remove(&date).unwrap_or_default();
            CalendarDay {
                date,
                booked: items.len() as u64,
                orders: items,
            }
        })
        .collect();
    Ok(Json(CalendarResp { month, days }))
}

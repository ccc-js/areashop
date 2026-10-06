use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::AuthUser,
    entities::{
        order::{next_status, OrderAction},
        order_item, product,
    },
    state::AppState,
};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct CreateOrder {
    #[validate(length(min = 1))]
    items: Vec<OrderLine>,
    #[validate(length(min = 1))]
    pickup_place: String,
    pickup_at: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct OrderLine {
    product_id: i32,
    #[validate(range(min = 1))]
    qty: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct OrderDetail {
    #[serde(flatten)]
    order: crate::entities::order::Model,
    items: Vec<order_item::Model>,
}

pub(crate) async fn create(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOrder>,
) -> Result<Json<OrderDetail>, (StatusCode, String)> {
    req.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 整單必須同一家店（面交 MVP 簡化；跨店請分開下單）
    let out =
        s.db.transaction::<_, OrderDetail, sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let mut shop_id: Option<i32> = None;
                let mut total = 0i32;
                let mut lines: Vec<(product::Model, i32)> = vec![];
                for line in &req.items {
                    let p = product::Entity::find_by_id(line.product_id)
                        .one(txn)
                        .await?
                        .ok_or(sea_orm::DbErr::RecordNotFound(format!(
                            "product {} not found",
                            line.product_id
                        )))?;
                    if p.status != "on" {
                        return Err(sea_orm::DbErr::Custom("product not on sale".into()));
                    }
                    if p.stock < line.qty {
                        return Err(sea_orm::DbErr::Custom(format!(
                            "stock not enough for {}",
                            p.title
                        )));
                    }
                    match shop_id {
                        None => shop_id = Some(p.shop_id),
                        Some(id) if id != p.shop_id => {
                            return Err(sea_orm::DbErr::Custom(
                                "multi-shop order not supported in v0.1".into(),
                            ))
                        }
                        _ => {}
                    }
                    total += p.price_cents * line.qty;
                    lines.push((p, line.qty));
                }
                let shop_id = shop_id.ok_or(sea_orm::DbErr::Custom("empty order".into()))?;
                // 扣庫存（事務內；sqlite/postgres 通用，不用 SELECT FOR UPDATE）
                for (p, qty) in &lines {
                    let mut am: product::ActiveModel = product::Entity::find_by_id(p.id)
                        .one(txn)
                        .await?
                        .ok_or(sea_orm::DbErr::RecordNotFound("product gone".into()))?
                        .into();
                    let left = p.stock - *qty;
                    if left < 0 {
                        return Err(sea_orm::DbErr::Custom("stock changed, retry".into()));
                    }
                    am.stock = Set(left);
                    if left == 0 {
                        am.status = Set("soldout".to_string());
                    }
                    am.update(txn).await?;
                }
                let om = crate::entities::order::ActiveModel {
                    buyer_id: Set(auth.id),
                    shop_id: Set(shop_id),
                    status: Set("pending".to_string()),
                    pickup_place: Set(req.pickup_place.clone()),
                    pickup_at: Set(req.pickup_at.clone()),
                    total_cents: Set(total),
                    remark: Set(req.remark.clone()),
                    created_at: Set(Utc::now()),
                    ..Default::default()
                };
                let order = om.insert(txn).await?;
                let mut items = vec![];
                for (p, qty) in &lines {
                    let im = order_item::ActiveModel {
                        order_id: Set(order.id),
                        product_id: Set(p.id),
                        qty: Set(*qty),
                        price_cents: Set(p.price_cents),
                        ..Default::default()
                    };
                    items.push(im.insert(txn).await?);
                }
                Ok(OrderDetail { order, items })
            })
        })
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(out))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ListQ {
    role: Option<String>, // buyer | seller
}

pub(crate) async fn list(
    State(s): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQ>,
) -> Result<Json<Vec<crate::entities::order::Model>>, (StatusCode, String)> {
    use crate::entities::{order, shop};
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
            order::Entity::find()
                .filter(order::Column::ShopId.is_in(ids))
                .all(&s.db)
                .await
                .map(Json)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
        _ => order::Entity::find()
            .filter(order::Column::BuyerId.eq(auth.id))
            .all(&s.db)
            .await
            .map(Json)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
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
    let items = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(order.id))
        .all(&s.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(OrderDetail { order, items }))
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
    action: String, // confirm | ready | complete | cancel
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

    // 權限：confirm/ready=店主；complete=雙方皆可；cancel=pending/confirmed 時雙方可
    match action {
        OrderAction::Confirm | OrderAction::Ready if !is_seller => {
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
    let next = next_status(&order.status, action)
        .ok_or((StatusCode::BAD_REQUEST, "illegal transition".to_string()))?;
    // 取消要回補庫存（事務）
    if next == "cancelled" {
        s.db.transaction::<_, (), sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                let items = order_item::Entity::find()
                    .filter(order_item::Column::OrderId.eq(order.id))
                    .all(txn)
                    .await?;
                for it in items {
                    if let Some(p) = product::Entity::find_by_id(it.product_id).one(txn).await? {
                        let mut am: product::ActiveModel = p.into();
                        // stock 回補：用當前值 + qty（事務內讀寫，MVP 足夠）
                        let cur = product::Entity::find_by_id(it.product_id)
                            .one(txn)
                            .await?
                            .unwrap();
                        am.stock = Set(cur.stock + it.qty);
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
    Ok(Json(m))
}

pub mod admin;
pub mod areas;
pub mod auth_routes;
pub mod items;
pub mod orders;
pub mod shops;

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::state::AppState;

pub fn v1_router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        // auth
        .route("/auth/register", post(auth_routes::register))
        .route("/auth/login", post(auth_routes::login))
        .route("/auth/me", get(auth_routes::me))
        // areas
        .route("/areas", get(areas::list))
        .route("/areas/counties", get(areas::counties))
        .route("/areas/townships", get(areas::townships))
        .route("/areas/search", get(areas::search))
        // shops
        .route("/shops", post(shops::create).get(shops::list_mine))
        .route("/shops/:id", get(shops::get_one).patch(shops::update))
        // items（統一項目：賣東西＋賣服務同一種，不分兩類）
        .route("/items", post(items::create).get(items::list))
        .route("/items/:id", get(items::get_one).patch(items::update))
        .route("/items/:id/rules", put(items::put_rules))
        .route("/items/:id/exceptions", post(items::post_exception))
        .route(
            "/items/:id/exceptions/:date",
            axum::routing::delete(items::delete_exception),
        )
        // orders（統一訂單：直接買＋選日期同一套；date 有值 = 預約）
        .route("/orders", post(orders::create).get(orders::list))
        .route(
            "/orders/:id",
            get(orders::get_one).patch(orders::transition),
        )
        // provider calendar（有日期的單，按日聚合）
        .route("/provider/calendar", get(orders::calendar))
        // admin（管理員介面；一覽唯讀，停權/下架沿用 PATCH）
        .route("/admin/areas", post(admin::create_area))
        .route(
            "/admin/areas/:id",
            put(admin::update_area).delete(admin::delete_area),
        )
        .route("/admin/shops", get(admin::list_shops))
        .route("/admin/items", get(admin::list_items))
        .route("/admin/users", get(admin::list_users))
        .route("/admin/orders", get(admin::list_orders))
}

async fn healthz() -> &'static str {
    "ok"
}

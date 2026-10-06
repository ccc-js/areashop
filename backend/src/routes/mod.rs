pub mod areas;
pub mod auth_routes;
pub mod orders;
pub mod products;
pub mod shops;

use axum::{
    routing::{get, post},
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
        // products
        .route("/products", post(products::create).get(products::list))
        .route(
            "/products/:id",
            get(products::get_one).patch(products::update),
        )
        // orders
        .route("/orders", post(orders::create).get(orders::list))
        .route(
            "/orders/:id",
            get(orders::get_one).patch(orders::transition),
        )
}

async fn healthz() -> &'static str {
    "ok"
}

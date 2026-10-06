use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub shop_id: i32,
    pub title: String,
    /// agri | poultry | beef | dumpling | other
    pub category: String,
    /// 以分為單位，避免浮點誤差
    pub price_cents: i32,
    pub stock: i32,
    pub unit: String,
    /// [{label, detail}] 面交地點，存 JSON（pg=JSONB / sqlite=TEXT+JSON）
    pub pickup_places: Json,
    pub pickup_note: Option<String>,
    pub images: Json,
    /// on | off | soldout
    pub status: String,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

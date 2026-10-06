use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "shops")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub owner_id: i32,
    pub area_id: i32,
    pub name: String,
    /// personal | store
    pub kind: String,
    pub description: Option<String>,
    pub address_text: Option<String>,
    pub phone: Option<String>,
    /// 實體店址（有店面才填；純面交賣家留空）
    pub address: Option<String>,
    /// 營業時間（有店面才填，如：週二至週日 09:00-18:00）
    pub opening_hours: Option<String>,
    /// 取貨方式：store（到店）| meetup（約面交）| both（皆可）
    pub pickup_mode: String,
    /// open | closed | banned
    pub status: String,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

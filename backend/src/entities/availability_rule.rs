use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 週範本：一條 = 某 weekday 的開關＋自填時間區段
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "availability_rules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub item_id: i32,
    /// 0=週日 .. 6=週六（同 chrono num_days_from_sunday）
    pub weekday: i32,
    pub open: bool,
    /// 時間區段文字陣列，如 ["09:00-12:00","14:00-18:00"]（只顯示不解讀）
    pub windows: Json,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

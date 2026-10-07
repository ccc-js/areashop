use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 單日例外：改時間一律寫這裡，不動週範本
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "availability_exceptions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub item_id: i32,
    /// YYYY-MM-DD（字串可比大小，跨 DB 通用）
    pub date: String,
    pub open: bool,
    /// 當日額滿（店家手動勾）：true 就算 open 也不給約
    pub full: bool,
    pub note: Option<String>,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

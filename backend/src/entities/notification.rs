use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 站內通知（輪詢版）：存參數不存句子，前端按 kind 組 i18n 文案
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    /// order_created | confirmed | ready | completed | cancelled | noshow
    pub kind: String,
    pub order_id: i32,
    /// 對方暱稱（下單者或店主，看方向）
    pub actor: String,
    pub read: bool,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 統一項目：賣東西和賣服務同一種模式。
/// - 賣東西：stock 有值（賣完為止），bookable 可關
/// - 賣服務：stock 為 NULL（不限量），bookable 開＋設可接案時間
/// - 混合：兩者都開（如下單買＋預約某天取）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub shop_id: i32,
    pub title: String,
    pub description: Option<String>,
    /// 以分為單位（可為 0，如免費諮詢）
    pub price_cents: i32,
    pub unit: String,
    pub images: Json,
    /// on | off | soldout
    pub status: String,
    /// 庫存；NULL = 不限量（服務型）
    pub stock: Option<i32>,
    /// 是否接受選日期預約
    pub bookable: bool,
    /// 取消期限：有日期的單，當日 00:00 前幾小時不可取消
    pub cancel_hours: i32,
    pub notice: Option<String>,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

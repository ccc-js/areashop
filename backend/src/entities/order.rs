use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buyer_id: i32,
    pub shop_id: i32,
    /// pending | confirmed | ready | completed | cancelled | noshow
    pub status: String,
    pub pickup_at: Option<String>,
    /// 預約日期 YYYY-MM-DD；None = 直接買（面交約時間）
    pub date: Option<String>,
    /// 區段文字（有日期時，買家從當日 windows 選一個，可空）
    pub window: Option<String>,
    pub total_cents: i32,
    pub remark: Option<String>,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ---- 統一訂單狀態機（商品＋預約同一套，與 DB 無關，可單元測試） ----
// 直接買：pending → confirmed → ready → completed
// 選日期：pending → confirmed → completed（可跳過 ready）
// 取消：pending/confirmed/ready → cancelled；爽約：confirmed/ready → noshow（店主記）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAction {
    Confirm,
    Ready,
    Complete,
    Cancel,
    Noshow,
}

pub fn next_status(current: &str, action: OrderAction) -> Option<&'static str> {
    match (current, action) {
        ("pending", OrderAction::Confirm) => Some("confirmed"),
        ("pending", OrderAction::Cancel) => Some("cancelled"),
        ("confirmed", OrderAction::Ready) => Some("ready"),
        ("confirmed", OrderAction::Complete) => Some("completed"),
        ("confirmed", OrderAction::Cancel) => Some("cancelled"),
        ("confirmed", OrderAction::Noshow) => Some("noshow"),
        ("ready", OrderAction::Complete) => Some("completed"),
        ("ready", OrderAction::Cancel) => Some("cancelled"),
        ("ready", OrderAction::Noshow) => Some("noshow"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_state_machine() {
        assert_eq!(
            next_status("pending", OrderAction::Confirm),
            Some("confirmed")
        );
        assert_eq!(next_status("pending", OrderAction::Ready), None);
        assert_eq!(next_status("confirmed", OrderAction::Ready), Some("ready"));
        assert_eq!(
            next_status("ready", OrderAction::Complete),
            Some("completed")
        );
        // 預約可跳過 ready 直接完成
        assert_eq!(
            next_status("confirmed", OrderAction::Complete),
            Some("completed")
        );
        assert_eq!(
            next_status("confirmed", OrderAction::Noshow),
            Some("noshow")
        );
        assert_eq!(next_status("pending", OrderAction::Noshow), None);
        assert_eq!(next_status("completed", OrderAction::Cancel), None);
    }
}

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub buyer_id: i32,
    pub shop_id: i32,
    /// pending | confirmed | ready | completed | cancelled
    pub status: String,
    pub pickup_place: String,
    pub pickup_at: Option<String>,
    pub total_cents: i32,
    pub remark: Option<String>,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ---- 訂單狀態機（與 DB 無關，可單元測試） ----
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAction {
    Confirm,
    Ready,
    Complete,
    Cancel,
}

pub fn next_status(current: &str, action: OrderAction) -> Option<&'static str> {
    match (current, action) {
        ("pending", OrderAction::Confirm) => Some("confirmed"),
        ("pending", OrderAction::Cancel) => Some("cancelled"),
        ("confirmed", OrderAction::Ready) => Some("ready"),
        ("confirmed", OrderAction::Cancel) => Some("cancelled"),
        ("ready", OrderAction::Complete) => Some("completed"),
        ("ready", OrderAction::Cancel) => Some("cancelled"),
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
        assert_eq!(next_status("completed", OrderAction::Cancel), None);
    }
}

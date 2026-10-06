use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "areas")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub county: String,
    pub township: String,
    pub village: Option<String>,
    pub center_lat: Option<f64>,
    pub center_lng: Option<f64>,
    pub radius_km: Option<f64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

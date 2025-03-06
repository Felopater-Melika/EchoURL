use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "urls")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub original_url: String,
    pub short_code: String,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use super::{
    components,
    db_ops::{DbModel, PvGetQuery, PvListQuery},
};
use sqlx::PgPool;

pub trait Prop:
    components::Component + DbModel<PvGetQuery, PvListQuery> + std::fmt::Debug
{
}

#[derive(Clone, Debug)]
pub struct PvBool {
    pub value: bool,
    pub page_id: i32,
    pub prop_id: i32,
}

impl Prop for PvBool {}

#[derive(Debug)]
pub struct Page {
    pub id: i32,
    pub collection_id: i32,
    pub title: String,
    pub props: Vec<Box<dyn Prop>>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: PgPool,
}

#[derive(Default, Clone, Debug)]
pub struct Item {
    pub id: Option<i32>,
    pub title: String,
    pub is_completed: bool,
}

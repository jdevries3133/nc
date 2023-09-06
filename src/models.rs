use super::{
    components,
    db_ops::{DbModel, PvGetQuery, PvListQuery},
};
use sqlx::PgPool;

/// A PropVal implementation corresponds with each `propval_*` table in the
/// database. It provides generic mechanisms for dealing with values on page
/// properties.
pub trait PropVal:
    components::Component + DbModel<PvGetQuery, PvListQuery> + std::fmt::Debug
{
    fn get_page_id(&self) -> i32;
    fn get_prop_id(&self) -> i32;
}

#[derive(Clone, Debug)]
pub struct PvBool {
    pub value: bool,
    pub page_id: i32,
    pub prop_id: i32,
}

impl PropVal for PvBool {
    fn get_page_id(&self) -> i32 {
        self.page_id
    }
    fn get_prop_id(&self) -> i32 {
        self.prop_id
    }
}

#[derive(Clone, Debug)]
pub struct PvInt {
    pub value: i64,
    pub page_id: i32,
    pub prop_id: i32,
}

impl PropVal for PvInt {
    fn get_page_id(&self) -> i32 {
        self.page_id
    }
    fn get_prop_id(&self) -> i32 {
        self.prop_id
    }
}

#[derive(Debug)]
pub struct Page {
    pub id: i32,
    pub collection_id: i32,
    pub title: String,
    pub props: Vec<Box<dyn PropVal>>,
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

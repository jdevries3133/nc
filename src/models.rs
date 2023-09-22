use super::{
    components,
    db_ops::{DbModel, PvGetQuery, PvListQuery},
};
use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Clone)]
pub struct Prop {
    pub id: i32,
    pub type_id: PropValTypes,
    pub collection_id: i32,
    pub name: String,
    pub order: i16,
}

/// This is only really used for adapting from `property.type_id` in the
/// database to one of our `Pv*` structs
#[derive(Copy, Clone, Debug)]
pub enum PropValTypes {
    Bool = 1,
    Int = 2,
    Float = 3,
    Str = 4,
    MultiString = 5,
    Date = 6,
    DateTime = 7,
}

/// Will panic if an invalid PropValTypes is passed in
pub fn propval_type_from_int(int: i32) -> PropValTypes {
    match int {
        1 => PropValTypes::Bool,
        2 => PropValTypes::Int,
        3 => PropValTypes::Float,
        4 => PropValTypes::Str,
        5 => PropValTypes::MultiString,
        6 => PropValTypes::Date,
        7 => PropValTypes::DateTime,
        _ => panic!("invalid prop-type {int}"),
    }
}

/// Get the `dyn PropVal` default value for any prop type
pub fn get_default(
    pv_type: PropValTypes,
    page_id: i32,
    prop_id: i32,
) -> Box<dyn PropVal> {
    match pv_type {
        PropValTypes::Bool => Box::new(PvBool {
            page_id,
            prop_id,
            value: false,
        }),
        PropValTypes::Int => Box::new(PvInt {
            page_id,
            prop_id,
            value: 0,
        }),
        _ => todo!("type {pv_type:?} not implemented"),
    }
}

/// A PropVal implementation corresponds with each `propval_*` table in the
/// database. It provides generic mechanisms for dealing with values on page
/// properties.
#[async_trait]
pub trait PropVal:
    components::Component + DbModel<PvGetQuery, PvListQuery> + std::fmt::Debug
{
    /// Get the existing database model using the `DbModel::get` method. If any
    /// error occurs (which is most likely caused by the row not currently
    /// existing), return a default model instead.

    // Implementers are a bit leaky, because it's possible that i.e, we get a
    // network error connecting to the database, and then read that as "does
    // not exist" and give the default back instead of propagating back the
    // underlying error. Fixing this is a bit tricky because I decided my
    // `DbModel::get` method would return `Result<T>` instead of
    // `Result<Option<T>>`, so there is no disambiguation between an error
    // (including not found), versus another type of error. So, I'll accept
    // the leakiness now, deferring a refactor to `Result<Option<T>>` in the
    // `DbModel` trait for later, which, when corrected, would affect all
    // implementation of this function.
    async fn get_or_init(db: &PgPool, query: &PvGetQuery) -> Self
    where
        Self: Sized;
    fn get_page_id(&self) -> i32;
    fn get_prop_id(&self) -> i32;
}

#[derive(Clone, Debug)]
pub struct PvBool {
    pub value: bool,
    pub page_id: i32,
    pub prop_id: i32,
}

#[async_trait]
impl PropVal for PvBool {
    async fn get_or_init(db: &PgPool, query: &PvGetQuery) -> Self
    where
        Self: Sized,
    {
        PvBool::get(db, query).await.unwrap_or(PvBool {
            page_id: query.page_id,
            prop_id: query.prop_id,
            value: false,
        })
    }
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

#[async_trait]
impl PropVal for PvInt {
    async fn get_or_init(db: &PgPool, query: &PvGetQuery) -> Self
    where
        Self: Sized,
    {
        PvInt::get(db, query).await.unwrap_or(PvInt {
            page_id: query.page_id,
            prop_id: query.prop_id,
            value: 0,
        })
    }
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
    pub content: Option<Content>,
}

#[derive(Debug)]
pub struct Content {
    pub page_id: i32,
    pub content: String,
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

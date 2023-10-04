use super::{
    components,
    db_ops::{DbModel, PvGetQuery, PvListQuery},
};
use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Debug, Clone)]
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
    Bool,
    Int,
    Float,
    Str,
    MultiString,
    Date,
    DateTime,
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

/// The string inside is the user-facing name of the filter type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterType {
    Eq(String),
    Neq(String),
    Gt(String),
    Lt(String),
    InRng(String),
    NotInRng(String),
    IsEmpty(String),
}

impl FilterType {
    pub fn new(id: i32, name: String) -> FilterType {
        match id {
            1 => FilterType::Eq(name),
            2 => FilterType::Neq(name),
            3 => FilterType::Gt(name),
            4 => FilterType::Lt(name),
            5 => FilterType::InRng(name),
            6 => FilterType::NotInRng(name),
            7 => FilterType::IsEmpty(name),
            _ => panic!("{id} is not a valid filter type"),
        }
    }
    pub fn get_display_name(&self) -> &str {
        match self {
            FilterType::Eq(_) => "Exactly Equals",
            FilterType::Neq(_) => "Does not Equal",
            FilterType::Gt(_) => "Is Greater Than",
            FilterType::Lt(_) => "Is Less Than",
            FilterType::InRng(_) => "Is Inside Range",
            FilterType::NotInRng(_) => "Is Not Inside Range",
            FilterType::IsEmpty(_) => "Is Empty",
        }
    }

    pub fn get_operator_str(&self) -> &'static str {
        match self {
            FilterType::Eq(_) => "=",
            FilterType::Gt(_) => ">",
            FilterType::Neq(_) => "!=",
            FilterType::Lt(_) => "<",
            // InRng // NotInRng do not map nicely into SQL operators
            _ => panic!("not supported"),
        }
    }

    pub fn get_int_repr(&self) -> i32 {
        match self {
            FilterType::Eq(_) => 1,
            FilterType::Neq(_) => 2,
            FilterType::Gt(_) => 3,
            FilterType::Lt(_) => 4,
            FilterType::InRng(_) => 5,
            FilterType::NotInRng(_) => 6,
            FilterType::IsEmpty(_) => 7,
        }
    }
    pub fn get_supported_filter_types(prop_type: PropValTypes) -> Vec<Self> {
        match prop_type {
            PropValTypes::Bool => vec![
                FilterType::Eq("Exactly Equals".into()),
                FilterType::Neq("Does not Equal".into()),
                FilterType::IsEmpty("Is Empty".into()),
            ],
            PropValTypes::Int => vec![
                FilterType::Eq("Exactly Equals".into()),
                FilterType::Gt("Does not Equal".into()),
                FilterType::Neq("Is Greater Than".into()),
                FilterType::Lt("Is Less Than".into()),
                FilterType::InRng("Is Inside Range".into()),
                FilterType::NotInRng("Is Not Inside Range".into()),
                FilterType::IsEmpty("Is Empty".into()),
            ],
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterBool {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub value: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterInt {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub value: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterIntRng {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub start: i64,
    pub end: i64,
}

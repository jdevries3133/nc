use super::great_enum_refactor;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct Prop {
    pub id: i32,
    pub type_id: great_enum_refactor::models::ValueType,
    pub collection_id: i32,
    pub name: String,
    pub order: i16,
}

/// Basically just needed for glue to get the old list page query moved
/// over to the new model. Maybe this will stay forever - who knows! Either
/// way, we're definitely cooking with enums now, baby.
#[derive(Debug)]
pub enum PvOrType {
    Pv(great_enum_refactor::models::PropVal),
    /// second item is `prop_id`
    Tp(great_enum_refactor::models::ValueType, i32),
}

#[derive(Debug)]
pub struct Page {
    pub id: i32,
    pub collection_id: i32,
    pub title: String,
    pub props: Vec<PvOrType>,
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
    pub fn get_supported_filter_types(
        prop_type: great_enum_refactor::models::ValueType,
    ) -> Vec<Self> {
        match prop_type {
            great_enum_refactor::models::ValueType::Bool => vec![
                FilterType::Eq("Exactly Equals".into()),
                FilterType::Neq("Does not Equal".into()),
                FilterType::IsEmpty("Is Empty".into()),
            ],
            great_enum_refactor::models::ValueType::Int => vec![
                FilterType::Eq("Exactly Equals".into()),
                FilterType::Gt("Does not Equal".into()),
                FilterType::Neq("Is Greater Than".into()),
                FilterType::Lt("Is Less Than".into()),
                FilterType::InRng("Is Inside Range".into()),
                FilterType::NotInRng("Is Not Inside Range".into()),
                FilterType::IsEmpty("Is Empty".into()),
            ],
            great_enum_refactor::models::ValueType::Float => vec![
                FilterType::Eq("Exactly Equals".into()),
                FilterType::Gt("Does not Equal".into()),
                FilterType::Neq("Is Greater Than".into()),
                FilterType::Lt("Is Less Than".into()),
                FilterType::InRng("Is Inside Range".into()),
                FilterType::NotInRng("Is Not Inside Range".into()),
                FilterType::IsEmpty("Is Empty".into()),
            ],
            great_enum_refactor::models::ValueType::Date => vec![
                FilterType::Eq("Exactly Equals".into()),
                FilterType::Gt("Does not Equal".into()),
                FilterType::Neq("Is Greater Than".into()),
                FilterType::Lt("Is Less Than".into()),
                FilterType::InRng("Is Inside Range".into()),
                FilterType::NotInRng("Is Not Inside Range".into()),
                FilterType::IsEmpty("Is Empty".into()),
            ],
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

#[derive(Clone, Debug)]
pub struct FilterFloat {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub value: f64,
}

#[derive(Clone, Debug)]
pub struct FilterFloatRng {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Debug)]
pub struct FilterDate {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub value: chrono::NaiveDate,
}

#[derive(Clone, Debug)]
pub struct FilterDateRng {
    pub id: i32,
    pub r#type: FilterType,
    pub prop_id: i32,
    pub start: chrono::NaiveDate,
    pub end: chrono::NaiveDate,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SortType {
    Asc,
    Desc,
}

impl SortType {
    pub fn from_int(n: i32) -> Result<Self> {
        match n {
            1 => Ok(SortType::Asc),
            2 => Ok(SortType::Desc),
            _ => bail!("unacceptable sort type"),
        }
    }
    pub fn get_int_repr(&self) -> i32 {
        match self {
            Self::Asc => 1,
            Self::Desc => 2,
        }
    }
    pub fn get_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// If `prop_id` or `type` are `None`, sorting is not currently enabled for
/// the collection.
#[derive(Debug, Eq, PartialEq)]
pub struct CollectionSort {
    pub collection_id: i32,
    pub prop_id: Option<i32>,
    pub r#type: Option<SortType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
}

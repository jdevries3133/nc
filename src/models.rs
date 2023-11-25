//! Core data-models for the application.

use super::prop_val;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Bool,
    Int,
    Float,
    Date,
    Datetime
}

impl ValueType {
    pub fn from_int(int: i32) -> Self {
        match int {
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Float,
            6 => Self::Date,
            7 => Self::Datetime,
            _ => panic!("{int} is not a valid ValueType"),
        }
    }
    pub fn of_value(value: &Value) -> Self {
        match value {
            Value::Int(_) => Self::Int,
            Value::Bool(_) => Self::Bool,
            Value::Date(_) => Self::Date,
            Value::Float(_) => Self::Float,
            Value::Datetime(_) => Self::Datetime
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Date(chrono::NaiveDate),
    Datetime(chrono::DateTime<chrono::Utc>)
}

impl Value {
    /// This is used for interpolating filter values into SQL queries. For
    /// now this is safe, because we're using strong data types which do not
    /// provide a vector for SQL injection attacks. Once we have string filters,
    /// I'm not sure how this will need to evolve to handle SQL injection
    /// safety, but I'll punt that problem for now since we only have
    /// primitive propvals at this point.
    pub fn as_sql(&self) -> String {
        match self {
            Self::Int(val) => format!("{val}"),
            Self::Bool(val) => {
                if *val {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Self::Date(val) => format!(r#"'{val}'"#),
            Self::Datetime(val) => format!(r#"'{val}'"#),
            Self::Float(val) => format!("{val}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Prop {
    pub id: i32,
    pub type_id: ValueType,
    pub collection_id: i32,
    pub name: String,
    pub order: i16,
}

/// Basically just needed for glue to get the old list page query moved
/// over to the new model. Maybe this will stay forever - who knows! Either
/// way, we're definitely cooking with enums now, baby.
#[derive(Debug)]
pub enum PvOrType {
    Pv(prop_val::models::PropVal),
    /// second item is `prop_id`
    Tp(ValueType, i32),
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

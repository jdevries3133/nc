use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use sqlx::{Pool, Postgres};

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Pool<Postgres>,
}

#[derive(Default, Clone, Debug)]
pub struct Item {
    pub id: Option<i32>,
    pub title: String,
    pub is_completed: bool,
}

#[derive(Clone, Debug)]
pub struct PvBool {
    pub value: bool,
}
#[derive(Clone, Debug)]
pub struct PvInt {
    pub value: i64,
}
#[derive(Clone, Debug)]
pub struct PvFloat {
    pub value: f64,
}
#[derive(Clone, Debug)]
pub struct PvStr {
    pub value: String,
}
#[derive(Clone, Debug)]
pub struct PvDate {
    pub value: NaiveDate,
}
#[derive(Clone, Debug)]
pub struct PvDateTime {
    pub value: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub enum PropVal {
    Boolean(PvBool),
    Int(PvInt),
    Float(PvFloat),
    String(PvStr),
    Date(PvDate),
    DateTime(PvDateTime),
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub page_id: i32,
    pub prop_id: i32,
    pub value: PropVal,
}

#[derive(Clone, Debug)]
pub struct PageSummary {
    pub id: i32,
    pub title: String,
    pub props: Vec<Prop>,
}

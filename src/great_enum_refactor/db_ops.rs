use super::{
    super::{
        db_ops::{DbModel, GetPropQuery},
        models::{Prop, PropValTypes},
    },
    models,
};
use anyhow::Result;
use async_trait::async_trait;
use futures::join;
use sqlx::{query, query_as, PgPool};

/// If the data-type is not known, we need to first query the `prop` table to
/// learn the data-type of the prop before we can know which `propval_*` table
/// needs to be queried to get the value. For PropVal queries, the caller can
/// provide `ValueType` if it is known, and this will save us a round-trip
/// to the database.
#[derive(Clone, Copy)]
pub enum ValueType {
    Bool,
    Int,
    Float,
    Date,
}

pub struct PvGetQuery {
    pub page_id: i32,
    pub prop_id: i32,
    pub data_type: Option<ValueType>,
}

pub struct PvListQuery {
    pub page_ids: Vec<i32>,
    pub data_type: Option<ValueType>,
}

struct Qres<T> {
    page_id: i32,
    prop_id: i32,
    value: T,
}

#[async_trait]
impl DbModel<PvGetQuery, PvListQuery> for models::PropVal {
    async fn get(db: &PgPool, query: &PvGetQuery) -> Result<Self> {
        let val_type = match query.data_type {
            Some(ty) => ty,
            None => {
                let prop =
                    Prop::get(db, &GetPropQuery { id: query.prop_id }).await?;
                // I'm just going to map this type from v1 into a type from v2 in the
                // hopes that it keeps both versions more separated as I work through
                // this refactor.
                match prop.type_id {
                    PropValTypes::Bool => ValueType::Bool,
                    PropValTypes::Int => ValueType::Int,
                    PropValTypes::Float => ValueType::Float,
                    PropValTypes::Date => ValueType::Date,
                    _ => todo!(),
                }
            }
        };
        let value = match val_type {
            ValueType::Bool => {
                let value = query_as!(
                    Qres::<bool>,
                    "select page_id, prop_id, value
                    from propval_bool
                    where page_id = $1 and prop_id = $2",
                    query.page_id,
                    query.prop_id
                )
                .fetch_one(db)
                .await?;
                models::Value::Bool(value.value)
            }
            ValueType::Int => {
                let value = query_as!(
                    Qres::<i64>,
                    "select page_id, prop_id, value
                    from propval_int
                    where page_id = $1 and prop_id = $2",
                    query.page_id,
                    query.prop_id
                )
                .fetch_one(db)
                .await?;
                models::Value::Int(value.value)
            }
            ValueType::Float => {
                let value = query_as!(
                    Qres::<f64>,
                    "select page_id, prop_id, value
                    from propval_float
                    where page_id = $1 and prop_id = $2",
                    query.page_id,
                    query.prop_id
                )
                .fetch_one(db)
                .await?;
                models::Value::Float(value.value)
            }
            ValueType::Date => {
                let value = query_as!(
                    Qres::<chrono::NaiveDate>,
                    "select page_id, prop_id, value
                    from propval_date
                    where page_id = $1 and prop_id = $2",
                    query.page_id,
                    query.prop_id
                )
                .fetch_one(db)
                .await?;
                models::Value::Date(value.value)
            }
        };
        Ok(models::PropVal {
            page_id: query.page_id,
            prop_id: query.prop_id,
            value,
        })
    }
    async fn list(db: &PgPool, query: &PvListQuery) -> Result<Vec<Self>> {
        let bools = query_as!(
            Qres::<bool>,
            "select page_id, prop_id, value
            from propval_bool
            where page_id = ANY($1)",
            &query.page_ids
        )
        .map(|row| models::PropVal {
            page_id: row.page_id,
            prop_id: row.prop_id,
            value: models::Value::Bool(row.value),
        })
        .fetch_all(db);

        let ints = query_as!(
            Qres::<i64>,
            "select page_id, prop_id, value
            from propval_int
            where page_id = ANY($1)",
            &query.page_ids
        )
        .map(|row| models::PropVal {
            page_id: row.page_id,
            prop_id: row.prop_id,
            value: models::Value::Int(row.value),
        })
        .fetch_all(db);

        let floats = query_as!(
            Qres::<f64>,
            "select page_id, prop_id, value
            from propval_float
            where page_id = ANY($1)",
            &query.page_ids
        )
        .map(|row| models::PropVal {
            page_id: row.page_id,
            prop_id: row.prop_id,
            value: models::Value::Float(row.value),
        })
        .fetch_all(db);

        let dates = query_as!(
            Qres::<chrono::NaiveDate>,
            "select page_id, prop_id, value
            from propval_date
            where page_id = ANY($1)",
            &query.page_ids
        )
        .map(|row| models::PropVal {
            page_id: row.page_id,
            prop_id: row.prop_id,
            value: models::Value::Date(row.value),
        })
        .fetch_all(db);

        let (bools, ints, floats, dates) = join!(bools, ints, floats, dates);

        let bools = bools?;
        let ints = ints?;
        let floats = floats?;
        let dates = dates?;

        let mut all_propvals = Vec::with_capacity(
            bools.len() + ints.len() + floats.len() + dates.len(),
        );
        all_propvals.extend_from_slice(&bools);
        all_propvals.extend_from_slice(&ints);
        all_propvals.extend_from_slice(&floats);
        all_propvals.extend_from_slice(&dates);

        Ok(all_propvals)
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        match self.value {
            models::Value::Bool(val) => {
                query!(
                    "insert into propval_bool (value, page_id, prop_id) values ($1, $2, $3)
                    on conflict (page_id, prop_id)
                    do update set value = $1",
                    val,
                    self.page_id,
                    self.prop_id
                ).execute(db).await?
            },
            models::Value::Int(val) => {
                query!(
                    "insert into propval_int (value, page_id, prop_id) values ($1, $2, $3)
                    on conflict (page_id, prop_id)
                    do update set value = $1",
                    val,
                    self.page_id,
                    self.prop_id
                ).execute(db).await?
            },
            models::Value::Float(val) => {
                query!(
                    "insert into propval_float (value, page_id, prop_id) values ($1, $2, $3)
                    on conflict (page_id, prop_id)
                    do update set value = $1",
                    val,
                    self.page_id,
                    self.prop_id
                ).execute(db).await?
            },
            models::Value::Date(val) => {
                query!(
                    "insert into propval_date (value, page_id, prop_id) values ($1, $2, $3)
                    on conflict (page_id, prop_id)
                    do update set value = $1",
                    val,
                    self.page_id,
                    self.prop_id
                ).execute(db).await?
            }
        };

        Ok(())
    }
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!()
    }
}

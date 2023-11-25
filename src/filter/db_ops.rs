use super::models;
use crate::{
    db_ops::DbModel,
    models::{Value, ValueType},
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::join;
use sqlx::{query, query_as, PgPool};

/// We're really into hackery here, and exposing a modeling mistake; whoopsies.
pub enum Variant {
    Single,
    Ranged,
}
pub struct GetFilterQuery {
    pub id: i32,
    pub value_type: ValueType,
    pub variant: Variant,
}

pub struct ListFilterQuery {
    pub collection_id: i32,
}

async fn get_primitive_filter(
    db: &PgPool,
    id: i32,
    r#type: ValueType,
) -> Result<models::Filter> {
    struct Qres<T> {
        id: i32,
        type_id: i32,
        prop_id: i32,
        value: T,
    }
    Ok(match r#type {
        ValueType::Int => {
            let res = query_as!(
                Qres::<i64>,
                "select id, type_id, prop_id, value
                        from filter_int f
                        where f.id = $1",
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Single(Value::Int(res.value)),
            }
        }
        ValueType::Bool => {
            let res = query_as!(
                Qres::<bool>,
                "select id, type_id, prop_id, value
                        from filter_bool f
                        where f.id = $1",
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Single(Value::Bool(res.value)),
            }
        }
        ValueType::Float => {
            let res = query_as!(
                Qres::<f64>,
                "select id, type_id, prop_id, value
                        from filter_float f
                        where f.id = $1",
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Single(Value::Float(res.value)),
            }
        }
        ValueType::Date => {
            let res = query_as!(
                Qres::<chrono::NaiveDate>,
                "select id, type_id, prop_id, value
                        from filter_date f
                        where f.id = $1",
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Single(Value::Date(res.value)),
            }
        }
    })
}

async fn get_ranged_filter(
    db: &PgPool,
    id: i32,
    r#type: ValueType,
) -> Result<models::Filter> {
    struct Qres<T> {
        id: i32,
        type_id: i32,
        prop_id: i32,
        start: T,
        end: T,
    }
    Ok(match r#type {
        ValueType::Int => {
            let res = query_as!(
                Qres::<i64>,
                r#"select id, type_id, prop_id, start, "end"
                from filter_int_range f
                where f.id = $1"#,
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Range(
                    Value::Int(res.start),
                    Value::Int(res.end),
                ),
            }
        }
        ValueType::Bool => {
            // Uh oh, go to Rust data modeling Jail, do not pass Go, do not
            // collect $200.
            panic!("I am sorry Rust, I have failed you");
        }
        ValueType::Date => {
            let res = query_as!(
                Qres::<chrono::NaiveDate>,
                r#"select id, type_id, prop_id, start, "end"
                from filter_date_range f
                where f.id = $1"#,
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Range(
                    Value::Date(res.start),
                    Value::Date(res.end),
                ),
            }
        }
        ValueType::Float => {
            let res = query_as!(
                Qres::<f64>,
                r#"select id, type_id, prop_id, start, "end"
                from filter_float_range f
                where f.id = $1"#,
                id
            )
            .fetch_one(db)
            .await?;
            models::Filter {
                id: res.id,
                prop_id: res.prop_id,
                r#type: models::FilterType::from_int(res.type_id),
                value: models::FilterValue::Range(
                    Value::Float(res.start),
                    Value::Float(res.end),
                ),
            }
        }
    })
}

#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::Filter {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        Ok(match query.variant {
            Variant::Single => {
                get_primitive_filter(db, query.id, query.value_type).await?
            }
            Variant::Ranged => {
                get_ranged_filter(db, query.id, query.value_type).await?
            }
        })
    }

    async fn list(db: &PgPool, query: &ListFilterQuery) -> Result<Vec<Self>> {
        struct Qres<T> {
            id: i32,
            prop_id: i32,
            r#type_id: i32,
            value: T,
        }
        let bools = query_as!(
            Qres::<bool>,
            "select f.id, f.prop_id, f.type_id, f.value
            from filter_bool f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Single(Value::Bool(row.value)),
        })
        .fetch_all(db);
        let ints = query_as!(
            Qres::<i64>,
            "select f.id, f.prop_id, f.type_id, f.value
            from filter_int f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Single(Value::Int(row.value)),
        })
        .fetch_all(db);
        struct QresRng<T> {
            id: i32,
            prop_id: i32,
            r#type_id: i32,
            start: T,
            end: T,
        }
        let int_ranges = query_as!(
            QresRng::<i64>,
            "select f.id, f.prop_id, f.type_id, f.start, f.end
            from filter_int_range f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Range(
                Value::Int(row.start),
                Value::Int(row.end),
            ),
        })
        .fetch_all(db);
        let floats = query_as!(
            Qres::<f64>,
            "select f.id, f.prop_id, f.type_id, f.value
            from filter_float f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Single(Value::Float(row.value)),
        })
        .fetch_all(db);
        let float_ranges = query_as!(
            QresRng::<f64>,
            "select f.id, f.prop_id, f.type_id, f.start, f.end
            from filter_float_range f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Range(
                Value::Float(row.start),
                Value::Float(row.end),
            ),
        })
        .fetch_all(db);
        let dates = query_as!(
            Qres::<chrono::NaiveDate>,
            "select f.id, f.prop_id, f.type_id, f.value
            from filter_date f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Single(Value::Date(row.value)),
        })
        .fetch_all(db);
        let date_ranges = query_as!(
            QresRng::<chrono::NaiveDate>,
            "select f.id, f.prop_id, f.type_id, f.start, f.end
            from filter_date_range f
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .map(|row| models::Filter {
            id: row.id,
            prop_id: row.prop_id,
            r#type: models::FilterType::from_int(row.type_id),
            value: models::FilterValue::Range(
                Value::Date(row.start),
                Value::Date(row.end),
            ),
        })
        .fetch_all(db);

        let (bools, ints, int_ranges, floats, float_ranges, dates, date_ranges) = join![
            bools,
            ints,
            int_ranges,
            floats,
            float_ranges,
            dates,
            date_ranges
        ];

        let mut bools = bools?;
        let mut ints = ints?;
        let mut int_ranges = int_ranges?;
        let mut floats = floats?;
        let mut float_ranges = float_ranges?;
        let mut dates = dates?;
        let mut date_ranges = date_ranges?;

        let mut result = Vec::with_capacity(
            bools.len()
                + ints.len()
                + int_ranges.len()
                + floats.len()
                + float_ranges.len()
                + dates.len()
                + date_ranges.len(),
        );

        result.append(&mut bools);
        result.append(&mut ints);
        result.append(&mut int_ranges);
        result.append(&mut floats);
        result.append(&mut float_ranges);
        result.append(&mut dates);
        result.append(&mut date_ranges);

        Ok(result)
    }

    async fn save(&self, db: &PgPool) -> Result<()> {
        match &self.value {
            models::FilterValue::Single(val) => match val {
                Value::Int(val) => {
                    query!(
                        "update filter_int set type_id = $1, value = $2
                        where id = $3",
                        self.r#type.get_int_repr(),
                        val,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                Value::Bool(val) => {
                    query!(
                        "update filter_bool set type_id = $1, value = $2
                        where id = $3",
                        self.r#type.get_int_repr(),
                        val,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                Value::Date(val) => {
                    query!(
                        "update filter_date set type_id = $1, value = $2
                        where id = $3",
                        self.r#type.get_int_repr(),
                        val,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                Value::Float(val) => {
                    query!(
                        "update filter_float set type_id = $1, value = $2
                        where id = $3",
                        self.r#type.get_int_repr(),
                        val,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
            },
            models::FilterValue::Range(v1, v2) => match (v1, v2) {
                (Value::Int(start), Value::Int(end)) => {
                    query!(
                        r#"update filter_int_range
                        set type_id = $1, start = $2, "end" = $3
                        where id = $4"#,
                        self.r#type.get_int_repr(),
                        start,
                        end,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                (Value::Float(start), Value::Float(end)) => {
                    query!(
                        r#"update filter_float_range
                        set type_id = $1, start = $2, "end" = $3
                        where id = $4"#,
                        self.r#type.get_int_repr(),
                        start,
                        end,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                (Value::Date(start), Value::Date(end)) => {
                    query!(
                        r#"update filter_date_range
                        set type_id = $1, start = $2, "end" = $3
                        where id = $4"#,
                        self.r#type.get_int_repr(),
                        start,
                        end,
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                (v1, v2) => {
                    bail!("{v1:?} and {v2:?} are different value types for ranged filter (save)");
                }
            },
        };

        Ok(())
    }

    async fn delete(self, db: &PgPool) -> Result<()> {
        match self.value {
            models::FilterValue::Single(val) => match val {
                Value::Int(_) => {
                    query!("delete from filter_int where id = $1", self.id)
                        .execute(db)
                        .await?;
                }
                Value::Bool(_) => {
                    query!("delete from filter_bool where id = $1", self.id)
                        .execute(db)
                        .await?;
                }
                Value::Date(_) => {
                    query!("delete from filter_date where id = $1", self.id)
                        .execute(db)
                        .await?;
                }
                Value::Float(_) => {
                    query!("delete from filter_float where id = $1", self.id)
                        .execute(db)
                        .await?;
                }
            },
            models::FilterValue::Range(v1, v2) => match (v1, v2) {
                (Value::Int(_), Value::Int(_)) => {
                    query!(
                        "delete from filter_int_range where id = $1",
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                (Value::Float(_), Value::Float(_)) => {
                    query!(
                        "delete from filter_float_range where id = $1",
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                (Value::Date(_), Value::Date(_)) => {
                    query!(
                        "delete from filter_float_range where id = $1",
                        self.id
                    )
                    .execute(db)
                    .await?;
                }
                (v1, v2) => {
                    bail!("{v1:?} and {v2:?} are different value types for ranged filter (delete)");
                }
            },
        };

        Ok(())
    }
}

pub async fn create_filter(
    db: &PgPool,
    prop_id: i32,
    filter_type: models::FilterType,
    value_type: ValueType,
) -> Result<models::Filter> {
    struct Qres {
        /// The ID of the newly created filter; we'll know the enum variants
        /// based on which branch we're in, and we are providing default values
        /// here.
        id: i32,
    }
    Ok(match filter_type {
        models::FilterType::Eq
        | models::FilterType::Lt
        | models::FilterType::Gt
        | models::FilterType::Neq
        | models::FilterType::IsEmpty => match value_type {
            ValueType::Int => {
                let new_id = query_as!(
                    Qres,
                    "insert into filter_int
                    (type_id, prop_id, value)
                    values
                    ($1, $2, $3)
                    returning id
                    ",
                    filter_type.get_int_repr(),
                    prop_id,
                    0
                )
                .fetch_one(db)
                .await?
                .id;
                models::Filter {
                    id: new_id,
                    prop_id,
                    r#type: filter_type,
                    value: models::FilterValue::Single(Value::Int(0)),
                }
            }
            ValueType::Bool => {
                let new_id = query_as!(
                    Qres,
                    "insert into filter_bool
                    (type_id, prop_id, value)
                    values
                    ($1, $2, $3)
                    returning id
                    ",
                    filter_type.get_int_repr(),
                    prop_id,
                    false
                )
                .fetch_one(db)
                .await?
                .id;
                models::Filter {
                    id: new_id,
                    prop_id,
                    r#type: filter_type,
                    value: models::FilterValue::Single(Value::Bool(false)),
                }
            }
            ValueType::Date => {
                let new_id = query_as!(
                    Qres,
                    "insert into filter_date
                    (type_id, prop_id, value)
                    values
                    ($1, $2, $3)
                    returning id
                    ",
                    filter_type.get_int_repr(),
                    prop_id,
                    chrono::Local::now().date_naive()
                )
                .fetch_one(db)
                .await?
                .id;
                models::Filter {
                    id: new_id,
                    prop_id,
                    r#type: filter_type,
                    value: models::FilterValue::Single(Value::Date(
                        chrono::Local::now().date_naive(),
                    )),
                }
            }
            ValueType::Float => {
                let new_id = query_as!(
                    Qres,
                    "insert into filter_float
                    (type_id, prop_id, value)
                    values
                    ($1, $2, $3)
                    returning id
                    ",
                    filter_type.get_int_repr(),
                    prop_id,
                    0.0
                )
                .fetch_one(db)
                .await?
                .id;
                models::Filter {
                    id: new_id,
                    prop_id,
                    r#type: filter_type,
                    value: models::FilterValue::Single(Value::Float(0.0)),
                }
            }
        },
        models::FilterType::InRng | models::FilterType::NotInRng => {
            match value_type {
                ValueType::Int => {
                    let new_id = query_as!(
                        Qres,
                        r#"insert into filter_int_range
                        (type_id, prop_id, start, "end")
                        values
                            ($1, $2, $3, $4)
                        returning id
                        "#,
                        filter_type.get_int_repr(),
                        prop_id,
                        0,
                        10
                    )
                    .fetch_one(db)
                    .await?
                    .id;
                    models::Filter {
                        id: new_id,
                        prop_id,
                        r#type: filter_type,
                        value: models::FilterValue::Range(
                            Value::Int(0),
                            Value::Int(10),
                        ),
                    }
                }
                ValueType::Date => {
                    let start = (chrono::Local::now()
                        - chrono::Duration::days(10))
                    .date_naive();
                    let end = chrono::Local::now().date_naive();
                    let new_id = query_as!(
                        Qres,
                        r#"insert into filter_date_range
                        (type_id, prop_id, start, "end")
                        values
                            ($1, $2, $3, $4)
                        returning id
                        "#,
                        filter_type.get_int_repr(),
                        prop_id,
                        start,
                        end
                    )
                    .fetch_one(db)
                    .await?
                    .id;
                    models::Filter {
                        id: new_id,
                        prop_id,
                        r#type: filter_type,
                        value: models::FilterValue::Range(
                            Value::Date(start),
                            Value::Date(end),
                        ),
                    }
                }
                ValueType::Float => {
                    let new_id = query_as!(
                        Qres,
                        r#"insert into filter_float_range
                        (type_id, prop_id, start, "end")
                        values
                            ($1, $2, $3, $4)
                        returning id
                        "#,
                        filter_type.get_int_repr(),
                        prop_id,
                        0.0,
                        10.0
                    )
                    .fetch_one(db)
                    .await?
                    .id;
                    models::Filter {
                        id: new_id,
                        prop_id,
                        r#type: filter_type,
                        value: models::FilterValue::Range(
                            Value::Float(0.0),
                            Value::Float(10.0),
                        ),
                    }
                }
                ValueType::Bool => {
                    panic!("boolean range filter does not exist")
                }
            }
        }
    })
}

pub async fn does_collection_have_capacity_for_additional_filters(
    db: &PgPool,
    collection_id: i32,
) -> Result<bool> {
    struct Qres {
        cnt: Option<i64>,
    }
    let res = query_as!(
        Qres,
        "select count(1) cnt from property p
        left join filter_bool fb on p.id = fb.prop_id
        left join filter_int fi on p.id = fi.prop_id
        left join filter_int_range fri on p.id = fri.prop_id
        left join filter_float ffl on p.id = ffl.prop_id
        left join filter_float_range fflr on p.id = fflr.prop_id
        left join filter_date fd on p.id = fd.prop_id
        left join filter_date_range fdr on p.id = fdr.prop_id
        where
            p.collection_id = $1
            and fb.id is null
            and fi.id is null
            and fri.id is null
            and ffl.id is null
            and fflr.id is null
            and fd.id is null
            and fdr.id is null
        ",
        collection_id
    )
    .fetch_one(db)
    .await?;

    let count = res
        .cnt
        .expect("idk... why would count(1) not return a count? WTF");

    Ok(count > 0)
}

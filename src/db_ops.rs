use super::models;
use anyhow::Result;
use chrono::prelude::*;
use futures::try_join;
use sqlx::{postgres::PgPool, query, query_as};
use std::collections::HashMap;

pub async fn get_items(
    db: &PgPool,
    page: Option<i32>,
) -> Result<Vec<models::Item>> {
    let page_size = 20;
    let offset: i64 = if let Some(p) = page {
        let p64: i64 = p.into();
        p64 * page_size
    } else {
        0
    };
    struct QRes {
        id: i32,
        title: String,
        is_completed: bool,
    }
    let res = query_as!(
        QRes,
        "select id, title, is_completed from item
        order by is_completed, id desc
        limit $1 offset $2",
        page_size,
        offset
    )
    .fetch_all(db)
    .await?;

    Ok(res
        .iter()
        .map(|i| models::Item {
            id: Some(i.id),
            title: i.title.to_owned(),
            is_completed: i.is_completed,
        })
        .collect())
}

pub async fn save_item(
    db: &PgPool,
    mut item: models::Item,
) -> Result<models::Item> {
    struct QRes {
        id: i32,
    }
    if let Some(id) = item.id {
        query!(
            "
            update item
            set
                title = $1,
                is_completed = $2
            where id = $3
            ",
            item.title,
            item.is_completed,
            id
        )
        .execute(db)
        .await?;
        Ok(item)
    } else {
        let res = query_as!(
            QRes,
            "
        insert into item (title, is_completed) values ($1, $2)
        returning id
        ",
            item.title,
            item.is_completed
        )
        .fetch_one(db)
        .await?;

        item.id = Some(res.id);

        Ok(item)
    }
}

pub async fn delete_item(db: &PgPool, id: i32) -> Result<()> {
    query!("delete from item where id = $1", id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_collection_name(db: &PgPool, id: i32) -> Result<String> {
    struct QRes {
        name: String,
    }
    let res = query_as!(QRes, "select name from collection where id = $1", id)
        .fetch_one(db)
        .await?;

    Ok(res.name)
}

pub async fn list_collection_pages(
    db: &PgPool,
    collection_id: i32,
    page: Option<i32>,
) -> Result<Vec<models::PageSummary>> {
    let limit: i64 = 100;
    let offset = if let Some(p) = page {
        i64::from(p) * limit
    } else {
        0
    };
    struct QResPage {
        id: i32,
        title: String,
    }
    let pages = query_as!(
        QResPage,
        "
        select id, title from page where collection_id = $1
        limit $2 offset $3
        ",
        collection_id,
        limit,
        offset
    )
    .fetch_all(db)
    .await?;
    let ids: Vec<i32> = pages.iter().map(|p| p.id).collect();
    let propvals = get_propvals(db, ids).await?;
    let pv_map: HashMap<i32, Vec<models::Prop>> =
        propvals.iter().fold(HashMap::new(), |mut acc, el| {
            let vec = acc.get_mut(&el.page_id);
            if let Some(vec) = vec {
                vec.push(el.clone())
            } else {
                acc.insert(el.page_id, vec![el.clone()]);
            }
            acc
        });
    Ok(pages
        .iter()
        .map(|p| models::PageSummary {
            id: p.id,
            title: p.title.clone(),
            props: if let Some(pv) = pv_map.get(&p.id) {
                pv.to_vec()
            } else {
                vec![]
            },
        })
        .collect::<Vec<models::PageSummary>>())
}

trait IntoProp {
    fn create_prop(&self) -> models::Prop;
}
macro_rules! def_propval_query_result {
    ($struct_name:ident, $type_name:ty, $pv_enum_variant_name:ident, $pv_model_name:ident) => {
        struct $struct_name {
            page_id: i32,
            prop_id: i32,
            value: $type_name,
        }
        impl IntoProp for $struct_name {
            fn create_prop(&self) -> models::Prop {
                models::Prop {
                    page_id: self.page_id,
                    prop_id: self.prop_id,
                    value: models::PropVal::$pv_enum_variant_name(
                        models::$pv_model_name {
                            value: self.value.clone(),
                        },
                    ),
                }
            }
        }
    };
}

def_propval_query_result!(QresBool, bool, Bool, PvBool);
def_propval_query_result!(QresInt, i64, Int, PvInt);
def_propval_query_result!(QresFloat, f64, Float, PvFloat);
def_propval_query_result!(QresStr, String, Str, PvStr);
def_propval_query_result!(QresDate, chrono::NaiveDate, Date, PvDate);

struct QresDatetime<Tz: TimeZone> {
    page_id: i32,
    prop_id: i32,
    value: chrono::DateTime<Tz>,
}
impl<Tz: TimeZone> IntoProp for QresDatetime<Tz> {
    fn create_prop(&self) -> models::Prop {
        models::Prop {
            page_id: self.page_id,
            prop_id: self.prop_id,
            value: models::PropVal::DateTime(models::PvDateTime {
                value: self.value.with_timezone(&Utc),
            }),
        }
    }
}

async fn get_propvals(
    db: &PgPool,
    page_ids: Vec<i32>,
) -> Result<Vec<models::Prop>> {
    // I think this could be made much less of a slog with some smart macros,
    // but I am not a macro wizard, and also I think this will be the most
    // hairy area in terms of dealing with propval dynamism (especially since
    // write paths can probably be more granular). I'm going to slog through
    // for now!
    let bools = query_as!(
        QresBool,
        "
        select page_id, prop_id, value from propval_bool
        where page_id = ANY($1)
        ",
        &page_ids
    )
    .fetch_all(db);

    let ints = query_as!(
        QresInt,
        "
        select page_id, prop_id, value from propval_int where page_id = any($1)
        ",
        &page_ids
    )
    .fetch_all(db);

    let floats = query_as!(
        QresFloat,
        "
        select page_id, prop_id, value from propval_float
        where page_id = any($1)
        ",
        &page_ids
    )
    .fetch_all(db);

    let strs = query_as!(
        QresStr,
        "
        select page_id, prop_id, value from propval_str
        where page_id = any($1)
        ",
        &page_ids
    )
    .fetch_all(db);

    let dates = query_as!(
        QresDate,
        "
        select page_id, prop_id, value from propval_date
        where page_id = any($1)
        ",
        &page_ids
    )
    .fetch_all(db);

    let datetimes = query_as!(
        QresDatetime,
        "
        select page_id, prop_id, value from propval_datetime
        where page_id = any($1)
        ",
        &page_ids
    )
    .fetch_all(db);

    let (bools, ints, floats, strs, dates, datetimes) =
        try_join!(bools, ints, floats, strs, dates, datetimes)?;

    let mut output: Vec<models::Prop> = vec![];
    bools.iter().for_each(|b| output.push(b.create_prop()));
    ints.iter().for_each(|b| output.push(b.create_prop()));
    floats.iter().for_each(|b| output.push(b.create_prop()));
    strs.iter().for_each(|b| output.push(b.create_prop()));
    dates.iter().for_each(|b| output.push(b.create_prop()));
    datetimes.iter().for_each(|b| output.push(b.create_prop()));

    Ok(output)
}

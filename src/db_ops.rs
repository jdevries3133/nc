use super::{models, models::Prop};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::{postgres::PgPool, query, query_as};

#[async_trait]
pub trait DbModel<GetQuery, ListQuery> {
    async fn get(db: &PgPool, query: &GetQuery) -> Result<Self>
    where
        Self: Sized;
    async fn list(db: &PgPool, query: &ListQuery) -> Result<Vec<Self>>
    where
        Self: Sized;
    async fn save(&self, db: &PgPool) -> Result<()>;
}

pub struct PvGetQuery {
    pub page_id: i32,
    pub prop_id: i32,
}

/// We are generally going to want to get all the props for a small set of
/// pages. For typical display purposes, we'd be gathering all prop values for
/// a set of 100 pages at a time.
///
/// Later, we can add `prop_ids: Vec<i32>` here as well, which would basically
/// allow the user to select a subset of columns, but we don't need that for
/// now.
pub struct PvListQuery {
    pub page_ids: Vec<i32>,
}

#[async_trait]
impl DbModel<PvGetQuery, PvListQuery> for models::PvBool {
    async fn get(db: &PgPool, query: &PvGetQuery) -> Result<Self> {
        Ok(query_as!(
            Self,
            "select page_id, prop_id, value from propval_bool
            where page_id = $1 and prop_id = $2",
            query.page_id,
            query.prop_id
        )
        .fetch_one(db)
        .await?)
    }
    async fn list(db: &PgPool, query: &PvListQuery) -> Result<Vec<Self>> {
        Ok(query_as!(
            Self,
            "select page_id, prop_id, value from propval_bool
            where page_id = any($1)",
            &query.page_ids
        )
        .fetch_all(db)
        .await?)
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            "update propval_bool set value = $1
            where page_id = $2 and prop_id = $3",
            self.value,
            self.page_id,
            self.prop_id
        )
        .execute(db)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl DbModel<PvGetQuery, PvListQuery> for models::PvInt {
    async fn get(db: &PgPool, query: &PvGetQuery) -> Result<Self> {
        Ok(query_as!(
            Self,
            "select page_id, prop_id, value from propval_int
            where page_id = $1 and prop_id = $2",
            query.page_id,
            query.prop_id
        )
        .fetch_one(db)
        .await?)
    }
    async fn list(db: &PgPool, query: &PvListQuery) -> Result<Vec<Self>> {
        Ok(query_as!(
            Self,
            "select page_id, prop_id, value from propval_int
            where page_id = any($1)",
            &query.page_ids
        )
        .fetch_all(db)
        .await?)
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            "update propval_int set value = $1
            where page_id = $2 and prop_id = $3",
            self.value,
            self.page_id,
            self.prop_id
        )
        .execute(db)
        .await?;
        Ok(())
    }
}

pub async fn list_pages(
    db: &PgPool,
    collection_id: i32,
    page_number: i32,
) -> Result<Vec<models::Page>> {
    let page_size = 100;
    let offset = page_number * page_size;
    struct Pages {
        id: i32,
        title: String,
        collection_id: i32,
    }
    let pages = query_as!(
        Pages,
        "select id, title, collection_id from page
        where collection_id = $1
        limit $2 offset $3
        ",
        collection_id,
        i64::from(page_size),
        i64::from(offset)
    )
    .fetch_all(db)
    .await?;
    let pv_query = PvListQuery {
        page_ids: pages.iter().map(|p| p.id).collect(),
    };
    let bool_props = models::PvBool::list(db, &pv_query).await?;
    let int_props = models::PvInt::list(db, &pv_query).await?;

    Ok(pages
        .iter()
        .map(|page| {
            // #feelsbad
            let my_bools = bool_props
                .iter()
                .filter(|p| p.page_id == page.id)
                .map(|p| p.clone())
                .collect::<Vec<models::PvBool>>();
            let my_ints = int_props
                .iter()
                .filter(|p| p.page_id == page.id)
                .map(|p| p.clone())
                .collect::<Vec<models::PvInt>>();
            let mut all: Vec<Box<dyn Prop>> = vec![];
            for b in my_bools {
                all.push(Box::new(b));
            }
            for i in my_ints {
                all.push(Box::new(i));
            }
            models::Page {
                id: page.id,
                collection_id: page.collection_id,
                title: page.title.clone(),
                props: all,
            }
        })
        .collect())
}

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

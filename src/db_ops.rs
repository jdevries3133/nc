use super::{config, models, models::PropVal};
use anyhow::{bail, Result};
use async_trait::async_trait;
use sqlx::{postgres::PgPool, query, query_as};
use std::collections::HashMap;

#[async_trait]
pub trait DbModel<GetQuery, ListQuery>: Sync + Send {
    /// Get exactly one object from the database, matching the query. WIll
    /// return an error variant if the item does not exist.
    async fn get(db: &PgPool, query: &GetQuery) -> Result<Self>
    where
        Self: Sized;
    /// Get a set of objects from the database, matching the contents of the
    /// list query type.
    async fn list(db: &PgPool, query: &ListQuery) -> Result<Vec<Self>>
    where
        Self: Sized;
    /// Persist the object to the database
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
            "insert into propval_bool (value, page_id, prop_id) values ($1, $2, $3)
            on conflict (page_id, prop_id)
            do update set value = $1",
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
            "insert into propval_int (value, page_id, prop_id) values ($1, $2, $3)
            on conflict (page_id, prop_id)
            do update set value = $1",
            self.value,
            self.page_id,
            self.prop_id
        )
        .execute(db)
        .await?;
        Ok(())
    }
}

pub struct GetPageQuery {
    pub id: i32,
}

pub struct ListPageQuery {
    pub collection_id: i32,
    pub page_number: i32,
}

#[async_trait]
impl DbModel<GetPageQuery, ListPageQuery> for models::Page {
    /// Note: `Page.props` are not selected by this query yet; we're returning
    /// an empty vec for now.
    async fn get(db: &PgPool, query: &GetPageQuery) -> Result<Self> {
        struct Qres {
            collection_id: i32,
            title: String,
            content: Option<String>,
        }
        let res = query_as!(
            Qres,
            r#"select
                p.collection_id collection_id, p.title title, pc.content as "content?"
            from page p
            left join page_content pc on pc.page_id = p.id
            where p.id = $1"#,
            query.id
        )
        .fetch_one(db)
        .await?;

        Ok(Self {
            id: query.id,
            title: res.title,
            collection_id: res.collection_id,
            props: vec![],
            content: res.content.map(|content| models::Content {
                page_id: query.id,
                content,
            }),
        })
    }
    async fn list(db: &PgPool, query: &ListPageQuery) -> Result<Vec<Self>> {
        list_pages(db, query.collection_id, query.page_number).await
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            "update page set title = $1 where id = $2",
            self.title,
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
}

pub struct GetDbModelQuery {
    pub page_id: i32,
}

#[async_trait]
impl DbModel<GetDbModelQuery, ()> for models::Content {
    async fn get(db: &PgPool, query: &GetDbModelQuery) -> Result<Self> {
        Ok(query_as!(
            Self,
            "select content, page_id from page_content where page_id = $1",
            query.page_id
        )
        .fetch_one(db)
        .await?)
    }
    async fn list(_db: &PgPool, _q: &()) -> Result<Vec<Self>> {
        todo!()
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            "insert into page_content (page_id, content) values ($1, $2)
            on conflict (page_id)
            do update set content = $2",
            self.page_id,
            self.content
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

    let collection_prop_set = get_prop_set(db, collection_id).await?;
    let pv_query = PvListQuery {
        page_ids: pages.iter().map(|p| p.id).collect(),
    };
    let bool_props = models::PvBool::list(db, &pv_query).await?;
    let int_props = models::PvInt::list(db, &pv_query).await?;

    // Let's build a hash map to facilitate applying this blob of all prop
    // values for all pages in the collection into an ordered set of prop values
    // for each page being rendered.
    //
    // The keys of the hash map will be a tuple of `(page_id, prop_id)`. This
    // is the natural key for all prop-values, allowing us to lookup arbitrary
    // prop values by identifier as we perform an outer loop over pages and
    // an inner loop over props.
    let mut prop_map: HashMap<(i32, i32), Box<dyn PropVal>> = HashMap::new();

    macro_rules! insert {
        ($propset:ident) => {
            for item in $propset {
                prop_map.insert((item.page_id, item.prop_id), Box::new(item));
            }
        };
    }

    insert!(bool_props);
    insert!(int_props);

    Ok(pages
        .iter()
        .map(|page| {
            let mut props = vec![];
            for collection_prop in &collection_prop_set[..] {
                if let Some(existing) =
                    prop_map.remove(&(page.id, collection_prop.id))
                {
                    props.push(existing)
                } else {
                    props.push(models::get_default(
                        collection_prop.type_id.clone(),
                        page.id,
                        collection_prop.id,
                    ))
                }
            }
            models::Page {
                props,
                id: page.id,
                collection_id: page.collection_id,
                title: page.title.clone(),
                content: None,
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

pub async fn create_page(
    db: &PgPool,
    collection_id: i32,
    title: &str,
) -> Result<()> {
    query!(
        "insert into page (collection_id, title) values ($1, $2)",
        collection_id,
        title
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn get_prop_set(
    db: &PgPool,
    collection_id: i32,
) -> Result<Vec<models::Prop>> {
    struct Qres {
        id: i32,
        type_id: i32,
        collection_id: i32,
        name: String,
    }
    let props = query_as!(
        Qres,
        "select id, type_id, collection_id, name
        from property
        where collection_id = $1",
        collection_id
    )
    .fetch_all(db)
    .await?;
    if props.len() > config::PROP_SET_MAX {
        bail!("Collection {collection_id} has too many props");
    } else {
        Ok(props
            .iter()
            .map(|p| models::Prop {
                id: p.id,
                collection_id: p.collection_id,
                type_id: models::propval_type_from_int(p.type_id),
                name: p.name.clone(),
            })
            .collect())
    }
}

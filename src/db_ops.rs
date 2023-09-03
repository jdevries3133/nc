use super::models;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::{postgres::PgPool, query, query_as};

#[async_trait]
pub trait DbModel<T>: Sized {
    async fn get(db: &PgPool, identifier: T) -> Result<Self>;
    async fn save(&self, db: &PgPool) -> Result<()>;
}

pub struct PropValueIdentifier {
    pub page_id: i32,
    pub prop_id: i32,
}

#[async_trait]
impl DbModel<PropValueIdentifier> for models::PvBool {
    async fn get(db: &PgPool, identifier: PropValueIdentifier) -> Result<Self> {
        struct Qres {
            value: bool,
        }
        let res = query_as!(
            Qres,
            "select value from propval_bool
            where page_id = $1 and prop_id = $2",
            identifier.page_id,
            identifier.prop_id
        )
        .fetch_one(db)
        .await?;
        Ok(models::PvBool {
            prop_id: identifier.prop_id,
            page_id: identifier.page_id,
            value: res.value,
        })
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

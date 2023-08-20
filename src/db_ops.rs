use super::models;
use anyhow::Result;
use sqlx::{postgres::PgPool, query, query_as};

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

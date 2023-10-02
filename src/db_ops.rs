use super::{config, config::PROP_SET_MAX, models, models::PropVal};
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::join;
use sqlx::{
    postgres::{PgPool, Postgres},
    query, query_as,
    query_builder::QueryBuilder,
    Row,
};

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

pub struct GetPropQuery {
    pub id: i32,
}

pub struct ListPropQuery {
    pub collection_id: Option<i32>,
    /// A set of orders to match with the `prop.order` column. Can be useful
    /// for selecting props within a range, or getting the props that are
    /// immediately adjacent to a known prop.
    // On reflection, we later _need_ to look up all the props to
    // re-render the prop ordering component. All the complexity of
    // this order_in property is unnecessary. I figure I will remove it, but
    // it also took me a second to figure out how this query builder API
    // works, so I'll just do an implementation that keeps this functionality
    // in use at the cost of actually being less efficient so that I have a
    // template to follow for the query builder.
    pub order_in: Option<Vec<i16>>,

    /// If provided, override collection_id and return these specific props.
    pub exact_ids: Option<Vec<i32>>,
}
#[derive(sqlx::FromRow)]
struct QresProp {
    id: i32,
    type_id: i32,
    collection_id: i32,
    name: String,
    order: i16,
}
impl QresProp {
    fn into_prop(self) -> models::Prop {
        models::Prop {
            id: self.id,
            collection_id: self.collection_id,
            name: self.name,
            order: self.order,
            type_id: models::propval_type_from_int(self.type_id),
        }
    }
}

#[async_trait]
impl DbModel<GetPropQuery, ListPropQuery> for models::Prop {
    async fn get(db: &PgPool, query: &GetPropQuery) -> Result<Self> {
        let raw_prop = query_as!(
            QresProp,
            r#"select id, type_id, collection_id, name, "order"
            from property
            where id = $1"#,
            query.id
        )
        .fetch_one(db)
        .await?;

        Ok(raw_prop.into_prop())
    }
    async fn list(db: &PgPool, query: &ListPropQuery) -> Result<Vec<Self>> {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"select id, type_id, collection_id, name, "order"
            from property "#,
        );

        if let Some(ref ids) = query.exact_ids {
            builder.push("where id in (");
            let mut sep = builder.separated(",");
            for id in ids {
                sep.push_bind(id);
            }
            builder.push(")");
        } else if let Some(collection_id) = query.collection_id {
            builder.push("where collection_id = ");
            builder.push_bind(collection_id);
        } else {
            bail!("exact_ids or collection_id must be provided");
        };

        if let Some(order_set) = &query.order_in {
            builder.push(r#" and "order" in ("#);
            let mut sep = builder.separated(",");
            for o in order_set {
                sep.push_bind(o);
            }
            builder.push(")");
        };

        let mut raw_props =
            builder.build_query_as::<QresProp>().fetch_all(db).await?;

        raw_props.sort_by_key(|p| p.order);
        if raw_props.len() > PROP_SET_MAX {
            bail!("too many props for collection {:?}", query.collection_id);
        };
        let props = raw_props.drain(..).map(|p| p.into_prop()).collect();

        Ok(props)
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            r#"update property set
                name = $1,
                "order" = $2
            where id = $3"#,
            self.name,
            self.order,
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
}

pub struct GetFilterQuery {
    pub id: i32,
}

pub struct ListFilterQuery {
    pub collection_id: i32,
}

#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterBool {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            value: bool,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.value
            from filter_bool f
            join filter_type ft on f.type_id = ft.id
            where f.id = $1",
            query.id
        )
        .fetch_one(db)
        .await?;

        let filter_type =
            models::FilterType::new(res.type_id, res.type_name.clone());

        Ok(Self {
            id: res.id,
            prop_id: res.prop_id,
            value: res.value,
            r#type: filter_type,
        })
    }
    async fn list(db: &PgPool, query: &ListFilterQuery) -> Result<Vec<Self>> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            value: bool,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.value
            from filter_bool f
            join filter_type ft on f.type_id = ft.id
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .fetch_all(db)
        .await?;

        Ok(res
            .iter()
            .map(|r| {
                let filter_type =
                    models::FilterType::new(r.type_id, r.type_name.clone());
                Self {
                    id: r.id,
                    prop_id: r.prop_id,
                    value: r.value,
                    r#type: filter_type,
                }
            })
            .collect())
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            "update filter_bool set value = $1, type_id = $2 where id = $3",
            self.value,
            self.r#type.get_int_repr(),
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
}
#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterInt {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            prop_id: i32,
            value: i64,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                f.prop_id,
                f.value
            from filter_int f
            join filter_type ft on f.type_id = ft.id
            where f.id = $1",
            query.id
        )
        .fetch_one(db)
        .await?;

        let filter_type = models::FilterType::new(res.type_id, "".into());

        Ok(Self {
            id: res.id,
            prop_id: res.prop_id,
            value: res.value,
            r#type: filter_type,
        })
    }
    async fn list(db: &PgPool, query: &ListFilterQuery) -> Result<Vec<Self>> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            value: i64,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.value
            from filter_int f
            join filter_type ft on f.type_id = ft.id
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .fetch_all(db)
        .await?;

        Ok(res
            .iter()
            .map(|r| {
                let filter_type =
                    models::FilterType::new(r.type_id, r.type_name.clone());
                Self {
                    id: r.id,
                    prop_id: r.prop_id,
                    value: r.value,
                    r#type: filter_type,
                }
            })
            .collect())
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            "update filter_int set value = $1, type_id = $2 where id = $3",
            self.value,
            self.r#type.get_int_repr(),
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
}
#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterIntRng {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            start: i64,
            end: i64,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.start,
                f.end
            from filter_int_range f
            join filter_type ft on f.type_id = ft.id
            where f.id = $1",
            query.id
        )
        .fetch_one(db)
        .await?;

        let filter_type =
            models::FilterType::new(res.type_id, res.type_name.clone());

        Ok(Self {
            id: res.id,
            prop_id: res.prop_id,
            start: res.start,
            end: res.end,
            r#type: filter_type,
        })
    }
    async fn list(db: &PgPool, query: &ListFilterQuery) -> Result<Vec<Self>> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            start: i64,
            end: i64,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.start,
                f.end
            from filter_int_range f
            join filter_type ft on f.type_id = ft.id
            join property p on p.id = f.prop_id
            where p.collection_id = $1",
            query.collection_id
        )
        .fetch_all(db)
        .await?;

        Ok(res
            .iter()
            .map(|r| {
                let filter_type =
                    models::FilterType::new(r.type_id, r.type_name.clone());
                Self {
                    id: r.id,
                    prop_id: r.prop_id,
                    start: r.start,
                    end: r.end,
                    r#type: filter_type,
                }
            })
            .collect())
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        query!(
            r#"update filter_int_range
            set start = $1, "end" = $2, type_id = $3
            where id = $4"#,
            self.start,
            self.end,
            self.r#type.get_int_repr(),
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
}

pub async fn get_filters(
    db: &PgPool,
    collection_id: i32,
) -> Result<(
    Vec<models::FilterBool>,
    Vec<models::FilterInt>,
    Vec<models::FilterIntRng>,
)> {
    let filter_query = ListFilterQuery { collection_id };
    let (filter_bool, filter_int, filter_int_rng) = join!(
        models::FilterBool::list(db, &filter_query),
        models::FilterInt::list(db, &filter_query),
        models::FilterIntRng::list(db, &filter_query),
    );
    let filter_bool = filter_bool?;
    let filter_int = filter_int?;
    let filter_int_rng = filter_int_rng?;

    Ok((filter_bool, filter_int, filter_int_rng))
}

async fn get_page_list_ctx(
    db: &PgPool,
    collection_id: i32,
) -> Result<(
    Vec<models::FilterBool>,
    Vec<models::FilterInt>,
    Vec<models::FilterIntRng>,
    Vec<models::Prop>,
)> {
    let (filters, collection_prop_set) = join!(
        get_filters(db, collection_id),
        get_prop_set(db, collection_id)
    );
    let (filter_bool, filter_int, filter_int_rng) = filters?;
    let collection_prop_set = collection_prop_set?;

    Ok((filter_bool, filter_int, filter_int_rng, collection_prop_set))
}

pub async fn list_pages(
    db: &PgPool,
    collection_id: i32,
    page_number: i32,
) -> Result<Vec<models::Page>> {
    let (filter_bool, filter_int, filter_int_rng, collection_prop_set) =
        get_page_list_ctx(db, collection_id).await?;

    let page_size = 100;
    let offset = page_number * page_size;

    let mut query = QueryBuilder::new("select ");

    let mut sep = query.separated(",");
    sep.push("page.id id");
    sep.push("page.title title");
    sep.push("page.collection_id collection_id");
    for prop in &collection_prop_set[..] {
        sep.push(format!("prop{}.value prop{}", prop.id, prop.id));
    }
    query.push(" from page ");

    for prop in &collection_prop_set[..] {
        let table_name = match prop.type_id {
            models::PropValTypes::Int => "propval_int",
            models::PropValTypes::Bool => "propval_bool",
            _ => todo!(),
        };
        query.push(format!(
                "left join {table} as prop{prop_id}
                on prop{prop_id}.page_id = page.id 
                and prop{prop_id}.prop_id = {prop_id} ",
                table = table_name,
                prop_id = prop.id
            ));
    }

    if !(filter_bool.is_empty()
        && filter_int.is_empty()
        && filter_int_rng.is_empty())
    {
        query.push("where ");

        let mut sep = query.separated(" and ");
        for filter in &filter_bool[..] {
            let prop_id = filter.prop_id;
            if let models::FilterType::IsEmpty(..) = filter.r#type {
                sep.push(format!("prop{prop_id}.value is null"));
            } else {
                let operator = &filter.r#type.get_operator_str();
                let value = if filter.value { "true" } else { "false " };
                // The value here is a boolean, not a user-input string, so I think that
                // direct interpolation without binding is safe.
                sep.push(format!("prop{prop_id}.value {operator} {value} "));
            }
        }
        for filter in &filter_int[..] {
            let prop_id = filter.prop_id;
            if let models::FilterType::IsEmpty(..) = filter.r#type {
                sep.push(format!("prop{prop_id}.value is null"));
            } else {
                let operator = &filter.r#type.get_operator_str();
                let value = filter.value;
                // The value here is a boolean, not a user-input string, so I think that
                // direct interpolation without binding is safe.
                sep.push(format!("prop{prop_id}.value {operator} {value} "));
            }
        }
        for filter in &filter_int_rng[..] {
            let prop_id = filter.prop_id;
            let start = filter.start;
            let end = filter.end;
            match &filter.r#type {
                models::FilterType::InRng(..) => {
                    // The value here is a boolean, not a user-input string, so I think that
                    // direct interpolation without binding is safe.
                    sep.push(format!("prop{prop_id}.value > {start} "));
                    sep.push(format!("prop{prop_id}.value < {end} "));
                }
                models::FilterType::NotInRng(..) => {
                    sep.push(format!("prop{prop_id}.value < {start} "));
                    sep.push(format!("prop{prop_id}.value > {end} "));
                }
                ty => panic!("type {ty:?} not supported for int range filters"),
            };
        }
    }

    query.push(format!(" limit {page_size} offset {offset} "));

    let res = query.build().fetch_all(db).await?;
    let pages: Vec<models::Page> = res
        .iter()
        .map(|row| {
            let id: i32 = row.get("id");
            let title: String = row.get("title");
            let collection_id: i32 = row.get("collection_id");
            let props: Vec<Box<dyn PropVal>> = collection_prop_set
                .iter()
                .map(|prop| {
                    let prop_alias = format!("prop{}", prop.id);
                    match prop.type_id {
                        models::PropValTypes::Int => {
                            let value =
                                row.try_get(&prop_alias as &str).unwrap_or(0);
                            Box::new(models::PvInt {
                                page_id: id,
                                prop_id: prop.id,
                                value,
                            }) as _
                        }
                        models::PropValTypes::Bool => {
                            let value = row
                                .try_get(&prop_alias as &str)
                                .unwrap_or(false);
                            Box::new(models::PvBool {
                                page_id: id,
                                prop_id: prop.id,
                                value,
                            }) as _
                        }
                        _ => todo!(),
                    }
                })
                .collect();

            models::Page {
                id,
                title,
                collection_id,
                props,
                content: None,
            }
        })
        .collect();

    Ok(pages)
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
        order: i16,
    }
    let mut props = query_as!(
        Qres,
        r#"select id, type_id, collection_id, name, "order"
        from property
        where collection_id = $1"#,
        collection_id
    )
    .fetch_all(db)
    .await?;
    props.sort_by_key(|p| p.order);
    if props.len() > config::PROP_SET_MAX {
        bail!("Collection {collection_id} has too many props");
    } else {
        Ok(props
            .drain(..)
            .map(|p| models::Prop {
                id: p.id,
                collection_id: p.collection_id,
                name: p.name,
                order: p.order,
                type_id: models::propval_type_from_int(p.type_id),
            })
            .collect())
    }
}

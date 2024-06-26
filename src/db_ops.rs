//! Database operations; squirrel code lives here.

use super::{config, config::PROP_SET_MAX, filter, models, prop_val, pw};
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::join;
use sqlx::{
    postgres::{PgPool, PgPoolOptions, Postgres},
    query, query_as,
    query_builder::QueryBuilder,
    Row,
};

/// Generic container for database IDs. For example, to be used with queries
/// returning (id).
struct Id {
    id: i32,
}

pub async fn create_pg_pool() -> Result<sqlx::Pool<sqlx::Postgres>> {
    let db_url = &std::env::var("DATABASE_URL")
        .expect("database url to be defined in the environment")[..];

    Ok(PgPoolOptions::new()
        // Postgres default max connections is 100, and we'll take 'em
        // https://www.postgresql.org/docs/current/runtime-config-connection.html
        .max_connections(80)
        .connect(db_url)
        .await?)
}

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
    /// Delete the record from the databse, which could of course cascade
    /// to related rows based on the rules in the database schema for this
    /// table.
    ///
    /// Delete will consume `self`.
    ///
    /// Most `.save` methods are implemented using update queries, under the
    /// assumption that the object already exists and we are just mutating it
    /// and then calling `.save` to persist the mutation. Deletion, then,
    /// would naturally invalidate these save queries.
    ///
    /// Additionally, a delete operation can trigger cascading deletion,
    /// so the existing record will often change structurally after deletion,
    /// because other rows around it will be deleted as well. The strategy
    /// for recovering from deletion will vary based on the object type,
    /// which is why the delete method consumes `self`.
    async fn delete(self, _db: &PgPool) -> Result<()>;
}

pub struct GetPageQuery {
    pub id: i32,
}

pub struct ListPageQuery;

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
    async fn list(_db: &PgPool, _query: &ListPageQuery) -> Result<Vec<Self>> {
        todo!()
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
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!()
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
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!()
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
            type_id: models::ValueType::from_int(self.type_id),
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
            if ids.is_empty() {
                bail!("do not pass an empty ID vec")
            };
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
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!()
    }
}

async fn get_page_list_ctx(
    db: &PgPool,
    collection_id: i32,
) -> Result<(
    Vec<filter::models::Filter>,
    Vec<models::Prop>,
    Option<models::CollectionSort>,
)> {
    let sort_query = GetSortQuery { collection_id };
    let filter_query = filter::db_ops::ListFilterQuery { collection_id };
    let (filters, collection_prop_set, sort_details) = join!(
        filter::models::Filter::list(db, &filter_query),
        get_prop_set(db, collection_id),
        models::CollectionSort::get(db, &sort_query)
    );
    let filters = filters?;
    let collection_prop_set = collection_prop_set?;

    // Implicitly treating error as not-found here
    let sort_details = if let Ok(d) = sort_details {
        Some(d)
    } else {
        None
    };

    Ok((filters, collection_prop_set, sort_details))
}

pub async fn list_pages(
    db: &PgPool,
    collection_id: i32,
    page_number: i32,
) -> Result<(Vec<models::Page>, Vec<models::Prop>)> {
    let (filters, collection_prop_set, sort_details) =
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
            models::ValueType::Int => "propval_int",
            models::ValueType::Bool => "propval_bool",
            models::ValueType::Float => "propval_float",
            models::ValueType::Date => "propval_date",
        };
        query.push(format!(
            "left join {table} as prop{prop_id}
                on prop{prop_id}.page_id = page.id 
                and prop{prop_id}.prop_id = {prop_id} ",
            table = table_name,
            prop_id = prop.id
        ));
    }

    if !filters.is_empty() {
        query.push("where ");

        let mut sep = query.separated(" and ");
        for filter in &filters[..] {
            let prop_id = filter.prop_id;
            match filter.r#type {
                filter::models::FilterType::Eq
                | filter::models::FilterType::Neq
                | filter::models::FilterType::Lt
                | filter::models::FilterType::Gt => {
                    if let filter::models::FilterValue::Single(val) =
                        &filter.value
                    {
                        let operator = &filter.r#type.get_operator_str();
                        let value = val.as_sql();
                        // The value here is a boolean, not a user-input string,
                        // so I think that direct
                        // interpolation without binding
                        // is safe.
                        sep.push(format!(
                            "prop{prop_id}.value {operator} {value} "
                        ));
                    } else {
                        panic!("these filter types should not have ranged value types");
                    }
                }
                filter::models::FilterType::IsEmpty => {
                    sep.push(format!("prop{prop_id}.value is null"));
                }
                filter::models::FilterType::InRng => {
                    if let filter::models::FilterValue::Range(v1, v2) =
                        &filter.value
                    {
                        let v1 = v1.as_sql();
                        let v2 = v2.as_sql();
                        sep.push(format!(
                            "prop{prop_id}.value > {v1} and prop{prop_id}.value < {v2} "
                        ));
                    } else {
                        panic!("these filter types should not have singular value types");
                    }
                }
                filter::models::FilterType::NotInRng => {
                    if let filter::models::FilterValue::Range(v1, v2) =
                        &filter.value
                    {
                        let v1 = v1.as_sql();
                        let v2 = v2.as_sql();
                        sep.push(format!(
                            "prop{prop_id}.value < {v1} and prop{prop_id}.value > {v2} "
                        ));
                    } else {
                        panic!("these filter types should not have singular value types");
                    }
                }
            }
        }
    };

    if let Some(sort) = sort_details {
        if let Some(ty) = sort.r#type {
            if let Some(prop) = sort.prop_id {
                let prop_id = prop;
                let order_name = ty.get_sql();
                query.push(format!(
                    " order by prop{prop_id}.value {order_name} "
                ));
            }
        }
    };

    query.push(format!(" limit {page_size} offset {offset} "));

    let res = query.build().fetch_all(db).await?;
    let pages: Vec<models::Page> = res
        .iter()
        .map(|row| {
            let id: i32 = row.get("id");
            let title: String = row.get("title");
            let collection_id: i32 = row.get("collection_id");
            let props: Vec<models::PvOrType> = collection_prop_set
                .iter()
                .map(|prop| {
                    let prop_alias = format!("prop{}", prop.id);
                    match prop.type_id {
                        models::ValueType::Int => {
                            if let Ok(value) = row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(
                                    prop_val::models::PropVal {
                                        page_id: id,
                                        prop_id: prop.id,
                                        value: models::Value::Int(value),
                                    },
                                )
                            } else {
                                models::PvOrType::Tp(
                                    models::ValueType::Int,
                                    prop.id,
                                )
                            }
                        }
                        models::ValueType::Bool => {
                            if let Ok(value) = row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(
                                    prop_val::models::PropVal {
                                        page_id: id,
                                        prop_id: prop.id,
                                        value: models::Value::Bool(value),
                                    },
                                )
                            } else {
                                models::PvOrType::Tp(
                                    models::ValueType::Bool,
                                    prop.id,
                                )
                            }
                        }
                        models::ValueType::Float => {
                            if let Ok(value) = row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(
                                    prop_val::models::PropVal {
                                        page_id: id,
                                        prop_id: prop.id,
                                        value: models::Value::Float(value),
                                    },
                                )
                            } else {
                                models::PvOrType::Tp(
                                    models::ValueType::Float,
                                    prop.id,
                                )
                            }
                        }
                        models::ValueType::Date => {
                            if let Ok(value) = row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(
                                    prop_val::models::PropVal {
                                        page_id: id,
                                        prop_id: prop.id,
                                        value: models::Value::Date(value),
                                    },
                                )
                            } else {
                                models::PvOrType::Tp(
                                    models::ValueType::Date,
                                    prop.id,
                                )
                            }
                        }
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

    Ok((pages, collection_prop_set))
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
                type_id: models::ValueType::from_int(p.type_id),
            })
            .collect())
    }
}

pub struct GetSortQuery {
    pub collection_id: i32,
}

#[async_trait]
impl DbModel<GetSortQuery, ()> for models::CollectionSort {
    async fn get(db: &PgPool, query: &GetSortQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            prop_id: Option<i32>,
            type_id: Option<i32>,
        }
        let res = query_as!(
            Qres,
            "select id, sort_by_prop_id prop_id, sort_type_id type_id
            from collection where id = $1",
            query.collection_id
        )
        .fetch_one(db)
        .await?;
        let sort_type = if let Some(t) = res.type_id {
            models::SortType::from_int(t)?
        } else {
            bail!("could not find sort type")
        };
        let sort_prop_id = if let Some(p) = res.prop_id {
            p
        } else {
            bail!("could not find sort prop")
        };

        Ok(Self {
            collection_id: res.id,
            prop_id: Some(sort_prop_id),
            r#type: Some(sort_type),
        })
    }
    async fn list(_db: &PgPool, _query: &()) -> Result<Vec<Self>> {
        todo!()
    }
    async fn save(&self, db: &PgPool) -> Result<()> {
        let tp = self.r#type.as_ref().map(|t| t.get_int_repr());
        query!(
            r#"
            update collection set
                sort_by_prop_id = $1,
                sort_type_id = $2
            where id = $3
            "#,
            self.prop_id,
            tp,
            self.collection_id
        )
        .execute(db)
        .await?;

        Ok(())
    }
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!()
    }
}

pub struct GetUserQuery<'a> {
    /// `identifier` can be a users username _or_ email
    pub identifier: &'a str,
}

#[async_trait]
impl DbModel<GetUserQuery<'_>, ()> for models::User {
    async fn get(db: &PgPool, query: &GetUserQuery) -> Result<Self> {
        Ok(query_as!(
            Self,
            "select id, username, email from users
            where username = $1 or email = $1",
            query.identifier
        )
        .fetch_one(db)
        .await?)
    }
    async fn list(_db: &PgPool, _query: &()) -> Result<Vec<Self>> {
        todo!()
    }
    async fn save(&self, _db: &PgPool) -> Result<()> {
        todo!()
    }
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!();
    }
}

pub async fn create_user(
    db: &PgPool,
    username: String,
    email: String,
    pw: &pw::HashedPw,
) -> Result<models::User> {
    let id = query_as!(
        Id,
        "insert into users (username, email, salt, digest) values ($1, $2, $3, $4)
        returning id",
        username, email, pw.salt, pw.digest
    ).fetch_one(db).await?;

    Ok(models::User {
        id: id.id,
        username,
        email,
    })
}

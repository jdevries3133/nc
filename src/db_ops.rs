use super::{config, config::PROP_SET_MAX, great_enum_refactor, models, pw};
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::join;
use sqlx::{
    postgres::{PgPool, Postgres},
    query, query_as,
    query_builder::QueryBuilder,
    Row,
};

/// Generic container for database IDs. For example, to be used with queries
/// returning (id).
struct Id {
    id: i32,
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
            type_id: great_enum_refactor::models::ValueType::from_int(
                self.type_id,
            ),
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
    async fn delete(self, _db: &PgPool) -> Result<()> {
        todo!()
    }
}

#[async_trait]
pub trait FilterDb: DbModel<GetFilterQuery, ListFilterQuery> {
    /// Create a filter with the default value and persist it in the database
    /// before returning it to the caller.
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self>
    where
        Self: Sized;
}

#[async_trait]
impl FilterDb for models::FilterBool {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let Id { id } = query_as!(
            Id,
            "insert into filter_bool (type_id, prop_id, value) values
            ($1, $2, $3)
        returning (id)
        ",
            r#type.get_int_repr(),
            prop_id,
            false
        )
        .fetch_one(db)
        .await?;

        Ok(models::FilterBool {
            id,
            prop_id,
            r#type,
            value: false,
        })
    }
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
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_bool where id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl FilterDb for models::FilterInt {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let Id { id } = query_as!(
            Id,
            "insert into filter_int (type_id, prop_id, value) values
            ($1, $2, $3)
            returning (id)
        ",
            r#type.get_int_repr(),
            prop_id,
            0
        )
        .fetch_one(db)
        .await?;

        Ok(models::FilterInt {
            id,
            prop_id,
            r#type,
            value: 0,
        })
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
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_int where id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl FilterDb for models::FilterIntRng {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let Id { id } = query_as!(
        Id,
        r#"insert into filter_int_range (type_id, prop_id, start, "end") values
            ($1, $2, $3, $4)
        returning (id)
        "#,
        r#type.get_int_repr(),
        prop_id,
        0,
        10
    )
    .fetch_one(db)
    .await?;

        Ok(models::FilterIntRng {
            id,
            prop_id,
            r#type,
            start: 0,
            end: 10,
        })
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
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_int_range where id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl FilterDb for models::FilterFloat {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let Id { id } = query_as!(
            Id,
            "insert into filter_float (type_id, prop_id, value) values
            ($1, $2, $3)
            returning (id)
        ",
            r#type.get_int_repr(),
            prop_id,
            0.0
        )
        .fetch_one(db)
        .await?;

        Ok(models::FilterFloat {
            id,
            prop_id,
            r#type,
            value: 0.0,
        })
    }
}

#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterFloat {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            prop_id: i32,
            value: f64,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                f.prop_id,
                f.value
            from filter_float f
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
            value: f64,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.value
            from filter_float f
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
            "update filter_float set value = $1, type_id = $2 where id = $3",
            self.value,
            self.r#type.get_int_repr(),
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_float where id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl FilterDb for models::FilterFloatRng {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let Id { id } = query_as!(
        Id,
        r#"insert into filter_float_range (type_id, prop_id, start, "end") values
            ($1, $2, $3, $4)
        returning (id)
        "#,
        r#type.get_int_repr(),
        prop_id,
        0.0,
        10.0
    )
    .fetch_one(db)
    .await?;

        Ok(models::FilterFloatRng {
            id,
            prop_id,
            r#type,
            start: 0.0,
            end: 10.0,
        })
    }
}

#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterFloatRng {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            start: f64,
            end: f64,
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
            from filter_float_range f
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
            start: f64,
            end: f64,
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
            from filter_float_range f
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
            r#"update filter_float_range
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
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_float_range where id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl FilterDb for models::FilterDate {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let Id { id } = query_as!(
            Id,
            "insert into filter_date (type_id, prop_id, value) values
            ($1, $2, $3)
            returning (id)
        ",
            r#type.get_int_repr(),
            prop_id,
            chrono::Local::now().date_naive()
        )
        .fetch_one(db)
        .await?;

        Ok(models::FilterDate {
            id,
            prop_id,
            r#type,
            value: chrono::Local::now().date_naive(),
        })
    }
}

#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterDate {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            prop_id: i32,
            value: chrono::NaiveDate,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                f.prop_id,
                f.value
            from filter_date f
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
            value: chrono::NaiveDate,
        }
        let res = query_as!(
            Qres,
            "select
                f.id,
                ft.id type_id,
                ft.name type_name,
                f.prop_id,
                f.value
            from filter_date f
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
            "update filter_date set value = $1, type_id = $2 where id = $3",
            self.value,
            self.r#type.get_int_repr(),
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_date where id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl FilterDb for models::FilterDateRng {
    async fn create(
        db: &PgPool,
        prop_id: i32,
        r#type: models::FilterType,
    ) -> Result<Self> {
        let start =
            (chrono::Local::now() - chrono::Duration::days(10)).date_naive();
        let end = chrono::Local::now().date_naive();
        let Id { id } = query_as!(
        Id,
        r#"insert into filter_date_range (type_id, prop_id, start, "end") values
            ($1, $2, $3, $4)
        returning (id)
        "#,
        r#type.get_int_repr(),
        prop_id,
        start,
        end
    )
    .fetch_one(db)
    .await?;

        Ok(models::FilterDateRng {
            id,
            prop_id,
            r#type,
            start,
            end,
        })
    }
}

#[async_trait]
impl DbModel<GetFilterQuery, ListFilterQuery> for models::FilterDateRng {
    async fn get(db: &PgPool, query: &GetFilterQuery) -> Result<Self> {
        struct Qres {
            id: i32,
            type_id: i32,
            type_name: String,
            prop_id: i32,
            start: chrono::NaiveDate,
            end: chrono::NaiveDate,
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
            from filter_date_range f
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
            start: chrono::NaiveDate,
            end: chrono::NaiveDate,
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
            from filter_date_range f
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
            r#"update filter_date_range
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
    async fn delete(self, db: &PgPool) -> Result<()> {
        query!("delete from filter_date_range where id = $1", self.id)
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
    Vec<models::FilterFloat>,
    Vec<models::FilterFloatRng>,
    Vec<models::FilterDate>,
    Vec<models::FilterDateRng>,
)> {
    let filter_query = ListFilterQuery { collection_id };
    let (
        filter_bool,
        filter_int,
        filter_int_rng,
        filter_float,
        filter_float_rng,
        filter_date,
        filter_date_rng,
    ) = join!(
        models::FilterBool::list(db, &filter_query),
        models::FilterInt::list(db, &filter_query),
        models::FilterIntRng::list(db, &filter_query),
        models::FilterFloat::list(db, &filter_query),
        models::FilterFloatRng::list(db, &filter_query),
        models::FilterDate::list(db, &filter_query),
        models::FilterDateRng::list(db, &filter_query)
    );
    let filter_bool = filter_bool?;
    let filter_int = filter_int?;
    let filter_int_rng = filter_int_rng?;
    let filter_float = filter_float?;
    let filter_float_rng = filter_float_rng?;
    let filter_date = filter_date?;
    let filter_date_rng = filter_date_rng?;

    Ok((
        filter_bool,
        filter_int,
        filter_int_rng,
        filter_float,
        filter_float_rng,
        filter_date,
        filter_date_rng,
    ))
}

async fn get_page_list_ctx(
    db: &PgPool,
    collection_id: i32,
) -> Result<(
    Vec<models::FilterBool>,
    Vec<models::FilterInt>,
    Vec<models::FilterIntRng>,
    Vec<models::FilterFloat>,
    Vec<models::FilterFloatRng>,
    Vec<models::FilterDate>,
    Vec<models::FilterDateRng>,
    Vec<models::Prop>,
    Option<models::CollectionSort>,
)> {
    let sort_query = GetSortQuery { collection_id };
    let (filters, collection_prop_set, sort_details) = join!(
        get_filters(db, collection_id),
        get_prop_set(db, collection_id),
        models::CollectionSort::get(db, &sort_query)
    );
    let (
        filter_bool,
        filter_int,
        filter_int_rng,
        filter_float,
        filter_float_rng,
        filter_date,
        filter_date_rng,
    ) = filters?;
    let collection_prop_set = collection_prop_set?;

    // Implicitly treating error as not-found here
    let sort_details = if let Ok(d) = sort_details {
        Some(d)
    } else {
        None
    };

    Ok((
        filter_bool,
        filter_int,
        filter_int_rng,
        filter_float,
        filter_float_rng,
        filter_date,
        filter_date_rng,
        collection_prop_set,
        sort_details,
    ))
}

pub async fn list_pages(
    db: &PgPool,
    collection_id: i32,
    page_number: i32,
) -> Result<(Vec<models::Page>, Vec<models::Prop>)> {
    let (
        filter_bool,
        filter_int,
        filter_int_rng,
        filter_float,
        filter_float_rng,
        filter_date,
        filter_date_rng,
        collection_prop_set,
        sort_details,
    ) = get_page_list_ctx(db, collection_id).await?;

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
            great_enum_refactor::models::ValueType::Int => "propval_int",
            great_enum_refactor::models::ValueType::Bool => "propval_bool",
            great_enum_refactor::models::ValueType::Float => "propval_float",
            great_enum_refactor::models::ValueType::Date => "propval_date",
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
        && filter_int_rng.is_empty()
        && filter_float.is_empty()
        && filter_float_rng.is_empty()
        && filter_date.is_empty()
        && filter_date_rng.is_empty())
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
        for filter in &filter_float[..] {
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
        for filter in &filter_float_rng[..] {
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
        for filter in &filter_date[..] {
            let prop_id = filter.prop_id;
            if let models::FilterType::IsEmpty(..) = filter.r#type {
                sep.push(format!("prop{prop_id}.value is null"));
            } else {
                let operator = &filter.r#type.get_operator_str();
                let value = filter.value.to_string();
                // The value here is a boolean, not a user-input string, so I think that
                // direct interpolation without binding is safe.
                sep.push(format!("prop{prop_id}.value {operator} '{value}' "));
            }
        }
        for filter in &filter_date_rng[..] {
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
                        great_enum_refactor::models::ValueType::Int => {
                            if let Ok(value) =
                                row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(great_enum_refactor::models::PropVal {
                                    page_id: id,
                                    prop_id: prop.id,
                                    value: great_enum_refactor::models::Value::Int(value)
                                })
                            } else {
                                models::PvOrType::Tp(great_enum_refactor::models::ValueType::Int, id)
                            }
                        }
                        great_enum_refactor::models::ValueType::Bool => {
                            if let Ok(value) =
                                row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(great_enum_refactor::models::PropVal {
                                    page_id: id,
                                    prop_id: prop.id,
                                    value: great_enum_refactor::models::Value::Bool(value)
                                })
                            } else {
                                models::PvOrType::Tp(great_enum_refactor::models::ValueType::Bool, id)
                            }
                        }
                        great_enum_refactor::models::ValueType::Float => {
                            if let Ok(value) =
                                row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(great_enum_refactor::models::PropVal {
                                    page_id: id,
                                    prop_id: prop.id,
                                    value: great_enum_refactor::models::Value::Float(value)
                                })
                            } else {
                                models::PvOrType::Tp(great_enum_refactor::models::ValueType::Float, id)
                            }
                        }
                        great_enum_refactor::models::ValueType::Date => {
                            if let Ok(value) =
                                row.try_get(&prop_alias as &str)
                            {
                                models::PvOrType::Pv(great_enum_refactor::models::PropVal {
                                    page_id: id,
                                    prop_id: prop.id,
                                    value: great_enum_refactor::models::Value::Date(value)
                                })
                            } else {
                                models::PvOrType::Tp(great_enum_refactor::models::ValueType::Date, id)
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
                type_id: great_enum_refactor::models::ValueType::from_int(
                    p.type_id,
                ),
            })
            .collect())
    }
}

pub struct GetFilterQuery {
    pub id: i32,
}

pub struct ListFilterQuery {
    pub collection_id: i32,
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

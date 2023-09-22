use super::{
    components,
    components::Component,
    db_ops,
    db_ops::DbModel,
    errors::ServerError,
    htmx, models,
    models::{AppState, PropVal},
};
use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Form,
};
use serde::Deserialize;

pub async fn root() -> impl IntoResponse {
    components::Page {
        title: "NC".to_string(),
        children: Box::new(components::TodoHome {}),
    }
    .render()
}

#[cfg(feature = "live_reload")]
#[derive(Deserialize)]
pub struct PongParams {
    pub poll_interval_secs: u64,
}
/// The client will reload when this HTTP long-polling route disconnects,
/// effectively implementing live-reloading.
#[cfg(feature = "live_reload")]
pub async fn pong(
    Query(PongParams { poll_interval_secs }): Query<PongParams>,
) -> impl IntoResponse {
    tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs))
        .await;
    "pong"
}

#[cfg(not(feature = "live_reload"))]
pub async fn pong() -> impl IntoResponse {
    "pong"
}

pub async fn get_htmx_js() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_str("text/javascript")
            .expect("We can insert text/javascript headers"),
    );
    headers.insert(
        "cache-control",
        HeaderValue::from_str("Cache-Control: public, max-age=31536000")
            .expect("we can set cache control header"),
    );
    (headers, include_str!("./htmx-1.9.4.vendor.js"))
}

#[derive(Deserialize)]
pub struct ListParams {
    pub page: Option<i32>,
}
pub async fn list_todos(
    State(AppState { db }): State<AppState>,
    Query(ListParams { page }): Query<ListParams>,
) -> Result<impl IntoResponse, ServerError> {
    let todos = db_ops::get_items(&db, page).await?;

    let next_page = if let Some(p) = page { p + 1 } else { 1 };

    Ok(components::ItemList {
        items: todos,
        next_page: Some(next_page),
    }
    .render())
}

#[derive(Deserialize)]
pub struct CreateForm {
    id: Option<i32>,
    title: String,
    // My browser is sending `is_completed: on` when the box is checked, or
    // it omits the property entirely otherwise. I'm going to hope that's
    // basically some sort of web standard and handle it here.
    is_completed: Option<String>,
}

pub async fn save_todo(
    State(AppState { db }): State<AppState>,
    Form(CreateForm {
        id,
        title,
        is_completed,
    }): Form<CreateForm>,
) -> Result<impl IntoResponse, ServerError> {
    let item = db_ops::save_item(
        &db,
        models::Item {
            id,
            title,
            is_completed: is_completed.is_some(),
        },
    )
    .await?;

    Ok(components::Item { item }.render())
}

pub async fn delete_todo(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    db_ops::delete_item(&db, id).await?;
    Ok("")
}

pub async fn get_collection(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServerError> {
    let name = db_ops::get_collection_name(&db, id).await?;
    Ok(if headers.contains_key("Hx-Request") {
        components::Collection { id, name }.render()
    } else {
        components::Page {
            title: format!("Workspace ({name})"),
            children: Box::new(components::Collection { id, name }),
        }
        .render()
    })
}

#[derive(Deserialize)]
pub struct CpQuery {
    page: Option<i32>,
}
pub async fn collection_pages(
    State(AppState { db }): State<AppState>,
    Query(CpQuery { page }): Query<CpQuery>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let pages =
        db_ops::list_pages(&db, collection_id, page.unwrap_or(0)).await?;

    Ok(components::PageList { pages: &pages }.render())
}

pub async fn collection_prop_order(
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id,
            order_in: None,
        },
    )
    .await?;

    Ok(components::Page {
        title: format!("Set Prop Order (collection {})", collection_id),
        children: Box::new(components::PropOrderForm { props }),
    }
    .render())
}

#[derive(Deserialize)]
pub struct PvbForm {
    value: Option<String>,
}

pub async fn save_pv_bool(
    State(AppState { db }): State<AppState>,
    Path((page_id, prop_id)): Path<(i32, i32)>,
    Form(PvbForm { value }): Form<PvbForm>,
) -> Result<impl IntoResponse, ServerError> {
    let mut pvb = models::PvBool::get_or_init(
        &db,
        &db_ops::PvGetQuery { prop_id, page_id },
    )
    .await;
    let new_val = value.is_some();
    if new_val != pvb.value {
        pvb.value = value.is_some();
        pvb.save(&db).await?;
    }

    Ok(pvb.render())
}

#[derive(Deserialize)]
pub struct PvIntForm {
    value: Option<i64>,
}
pub async fn save_pv_int(
    State(AppState { db }): State<AppState>,
    Path((page_id, prop_id)): Path<(i32, i32)>,
    Form(PvIntForm { value }): Form<PvIntForm>,
) -> Result<impl IntoResponse, ServerError> {
    let mut existing = models::PvInt::get_or_init(
        &db,
        &db_ops::PvGetQuery { prop_id, page_id },
    )
    .await;

    if let Some(v) = value {
        if v != existing.value {
            existing.value = v;
            existing.save(&db).await?;
        }
    };
    Ok(existing.render())
}

pub async fn increment_prop_order(
    State(AppState { db }): State<AppState>,
    Path((collection_id, prop_id)): Path<(i32, i32)>,
) -> Result<impl IntoResponse, ServerError> {
    let mut prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: prop_id }).await?;
    let mut next_props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id,
            order_in: Some(vec![prop.order - 1]),
        },
    )
    .await?;
    if next_props.is_empty() {
        // So, as I point out near the definition of ListPropQuery, this final
        // query is basically wasteful. It would make more sense to just get all
        // the props up-front, but I'm doing this purely because I want to retain
        // the headway gained in setting up the query builder, and I'm figuring
        // on remove this once I have another real use-case for the query
        // builder.
        let all_props = models::Prop::list(
            &db,
            &db_ops::ListPropQuery {
                collection_id,
                order_in: None,
            },
        )
        .await?;

        return Ok(components::PropOrderForm { props: all_props }.render());
    };

    if next_props.len() != 1 {
        // If we've got more than one prop in the same collection with the same
        // order, we've encountered a data-integrity invariant, so we will
        // panic.
        panic!("collection {collection_id} did not have exactly one prop");
    };
    let mut next_prop = next_props
        .pop()
        .expect("we have exactly 1 prop in next_props");

    prop.order -= 1;
    next_prop.order += 1;

    // Soooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo
    // it would be very nice to have transactions here. But obviously, there
    // is a problem -- my DbModel abstraction does not support transactions.
    //
    // Hm, that's a dilemma.
    //
    // I gotta figure that one out.
    //
    // Later, though...
    prop.save(&db).await?;
    next_prop.save(&db).await?;

    // So, as I point out near the definition of ListPropQuery, this final
    // query is basically wasteful. It would make more sense to just get all
    // the props up-front, but I'm doing this purely because I want to retain
    // the headway gained in setting up the query builder, and I'm figuring
    // on remove this once I have another real use-case for the query
    // builder.
    let all_props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id,
            order_in: None,
        },
    )
    .await?;

    Ok(components::PropOrderForm { props: all_props }.render())
}

pub async fn decrement_prop_order(
    State(AppState { db }): State<AppState>,
    Path((collection_id, prop_id)): Path<(i32, i32)>,
) -> Result<impl IntoResponse, ServerError> {
    let mut prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: prop_id }).await?;
    let mut next_props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id,
            order_in: Some(vec![prop.order + 1]),
        },
    )
    .await?;
    if next_props.is_empty() {
        let all_props = models::Prop::list(
            &db,
            &db_ops::ListPropQuery {
                collection_id,
                order_in: None,
            },
        )
        .await?;
        return Ok(components::PropOrderForm { props: all_props }.render());
    };

    if next_props.len() != 1 {
        panic!("collection {collection_id} did not have exactly one prop");
    };
    let mut next_prop = next_props
        .pop()
        .expect("we have exactly 1 prop in next_props");

    prop.order += 1;
    next_prop.order -= 1;
    prop.save(&db).await?;
    next_prop.save(&db).await?;
    let all_props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id,
            order_in: None,
        },
    )
    .await?;

    Ok(components::PropOrderForm { props: all_props }.render())
}

pub async fn new_page_form(
    Path(collection_id): Path<i32>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let form = components::NewPage {
        collection_id,
        page_id: None,
        title: None,
    };
    if headers.contains_key("Hx-Request") {
        form.render()
    } else {
        components::Page {
            children: Box::new(form),
            title: "New Page".to_string(),
        }
        .render()
    }
}

pub async fn existing_page_form(
    State(AppState { db }): State<AppState>,
    Path(page_id): Path<i32>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServerError> {
    let page =
        models::Page::get(&db, &db_ops::GetPageQuery { id: page_id }).await?;

    Ok(if headers.contains_key("Hx-Request") {
        components::PageOverview { page: &page }.render()
    } else {
        components::Page {
            title: format!("{}", page.title),
            children: Box::new(components::PageOverview { page: &page }),
        }
        .render()
    })
}

#[derive(Deserialize)]
pub struct PageFormSubmission {
    id: i32,
    collection_id: i32,
    title: String,
}
pub async fn save_existing_page_form(
    State(AppState { db }): State<AppState>,
    Form(form): Form<PageFormSubmission>,
) -> Result<impl IntoResponse, ServerError> {
    let page = models::Page {
        id: form.id,
        collection_id: form.collection_id,
        title: form.title,
        props: vec![],
        content: None,
    };
    page.save(&db).await?;

    Ok(components::PageForm { page: &page }.render())
}

#[derive(Debug, Deserialize)]
pub struct PageForm {
    pub id: Option<i32>,
    pub title: String,
}
pub async fn handle_page_submission(
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
    Form(form): Form<PageForm>,
) -> Result<impl IntoResponse, ServerError> {
    if let Some(id) = form.id {
        models::Page {
            id,
            collection_id,
            title: form.title,
            props: Vec::new(),
            content: None,
        }
        .save(&db)
        .await?;
    } else {
        db_ops::create_page(&db, collection_id, &form.title).await?;
    }
    Ok((
        axum::http::StatusCode::CREATED,
        htmx::redirect(&format!("/collection/{collection_id}")),
        "OK",
    ))
}

pub async fn get_content_form(
    State(AppState { db }): State<AppState>,
    Path(page_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let existing_content =
        models::Content::get(&db, &db_ops::GetDbModelQuery { page_id }).await;
    if let Ok(c) = existing_content {
        Ok(c.render())
    } else {
        Ok(models::Content {
            page_id,
            content: "".to_string(),
        }
        .render())
    }
}

#[derive(Deserialize)]
pub struct ContentForm {
    content: String,
}
pub async fn handle_content_submission(
    State(AppState { db }): State<AppState>,
    Path(page_id): Path<i32>,
    Form(ContentForm { content }): Form<ContentForm>,
) -> Result<impl IntoResponse, ServerError> {
    let content = models::Content { page_id, content };
    content.save(&db).await?;
    Ok(components::ContentDisplay {
        page_id,
        content: Some(&content),
    }
    .render())
}

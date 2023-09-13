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
) -> Result<impl IntoResponse, ServerError> {
    let name = db_ops::get_collection_name(&db, id).await?;
    Ok(components::Page {
        title: format!("Workspace ({name})"),
        children: Box::new(components::Collection { id, name }),
    }
    .render())
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

    Ok(pages
        .iter()
        .map(|p| p.render())
        .collect::<Vec<String>>()
        .join(""))
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

pub async fn new_page_form(
    Path(collection_id): Path<i32>,
) -> impl IntoResponse {
    let form = components::NewPage {
        collection_id,
        page_id: None,
        title: None,
    };
    components::Page {
        children: Box::new(form),
        title: "New Page".to_string(),
    }
    .render()
}

pub async fn existing_page_form(
    State(AppState { db }): State<AppState>,
    Path(page_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let page =
        models::Page::get(&db, &db_ops::GetPageQuery { id: page_id }).await?;

    Ok(components::Page {
        title: format!("{}", page.title),
        children: Box::new(page),
    }
    .render())
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

pub async fn new_block_form() -> impl IntoResponse {
    components::BlockEditor {}.render()
}

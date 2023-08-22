use super::{
    components, components::Component, db_ops, errors::ServerError, models,
    models::AppState,
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
        title: format!("Workspace ({})", name),
        children: Box::new(components::Collection { id, name }),
    }
    .render())
}

#[derive(Deserialize)]
pub struct CpQuery {
    page: Option<i32>,
}
#[axum_macros::debug_handler]
pub async fn collection_pages(
    State(AppState { db }): State<AppState>,
    Query(CpQuery { page }): Query<CpQuery>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let pages = db_ops::list_collection_pages(&db, collection_id, page).await?;

    Ok(components::DbView { pages: &pages }.render())
}

pub async fn get_propval_form(
    State(AppState { db }): State<AppState>,
    Path((pv_type_id, prop_id, page_id)): Path<(i32, i32, i32)>,
) -> Result<impl IntoResponse, ServerError> {
    let prop = db_ops::get_prop(&db, pv_type_id, prop_id, page_id).await?;
    Ok(components::EditProp { prop: &prop }.render())
}

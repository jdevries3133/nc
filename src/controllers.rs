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
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Form,
};
use futures::join;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub async fn root(headers: HeaderMap) -> impl IntoResponse {
    if headers.contains_key("Hx-Request") {
        components::TodoHome {}.render()
    } else {
        components::Page {
            title: "NC".to_string(),
            children: Box::new(components::TodoHome {}),
        }
        .render()
    }
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
    (headers, include_str!("./htmx-1.9.6.vendor.js"))
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

    Ok(components::PageList {
        pages: &pages,
        collection_id,
    }
    .render())
}

pub async fn collection_prop_order(
    headers: HeaderMap,
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id: Some(collection_id),
            order_in: None,
            exact_ids: None,
        },
    )
    .await?;

    Ok(if headers.contains_key("Hx-Request") {
        components::PropOrderForm { props }.render()
    } else {
        components::Page {
            title: format!("Set Prop Order (collection {})", collection_id),
            children: Box::new(components::PropOrderForm { props }),
        }
        .render()
    })
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
            collection_id: Some(collection_id),
            order_in: Some(vec![prop.order - 1]),
            exact_ids: None,
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
                collection_id: Some(collection_id),
                order_in: None,
                exact_ids: None,
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
            collection_id: Some(collection_id),
            order_in: None,
            exact_ids: None,
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
            collection_id: Some(collection_id),
            order_in: Some(vec![prop.order + 1]),
            exact_ids: None,
        },
    )
    .await?;
    if next_props.is_empty() {
        let all_props = models::Prop::list(
            &db,
            &db_ops::ListPropQuery {
                collection_id: Some(collection_id),
                order_in: None,
                exact_ids: None,
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
            collection_id: Some(collection_id),
            order_in: None,
            exact_ids: None,
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

pub async fn get_filter_toolbar(
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let (bool_filters, int_filters, int_rng_filters) =
        db_ops::get_filters(&db, collection_id).await?;
    if bool_filters.is_empty()
        && int_filters.is_empty()
        && int_rng_filters.is_empty()
    {
        return Ok(components::EmptyFilterToolbar { collection_id }.render());
    };
    let mut all_props: Vec<i32> = Vec::with_capacity(
        bool_filters.len() + int_filters.len() + int_rng_filters.len(),
    );
    for f in &bool_filters[..] {
        all_props.push(f.prop_id);
    }
    for f in &int_filters[..] {
        all_props.push(f.prop_id);
    }
    for f in &int_rng_filters[..] {
        all_props.push(f.prop_id);
    }
    let prop_query = db_ops::ListPropQuery {
        collection_id: None,
        exact_ids: Some(all_props),
        order_in: None,
    };
    let mut props = models::Prop::list(&db, &prop_query).await?;
    let prop_by_id = props.drain(..).fold(HashMap::new(), |mut acc, prop| {
        acc.insert(prop.id, prop);
        acc
    });
    let get_prop_name = |prop_id: i32| {
        &prop_by_id
            .get(&prop_id)
            .expect("you lookup a prop that exists")
            .name as &str
    };
    Ok(components::FilterToolbar {
        collection_id,
        bool_filters,
        int_filters,
        int_rng_filters,
        get_prop_name: &get_prop_name,
    }
    .render())
}

// This needs to be async because axum requires route handlers to be async.
pub async fn hide_filter_toolbar(Path(collection_id): Path<i32>) -> String {
    components::FilterToolbarPlaceholder { collection_id }.render()
}

pub async fn get_bool_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterBool::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterBool {
        filter,
        prop_name: &related_prop.name,
    }
    .render())
}

pub async fn get_bool_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterBool::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::BoolFilterForm {
        filter,
        prop_name: &related_prop.name,
    }
    .render())
}

fn trigger_event(
    mut headers: HeaderMap,
    event_name: &'static str,
) -> HeaderMap {
    if headers.contains_key("Hx-Trigger") {
        let val = headers.get("Hx-Trigger").expect("we know it's here");
        let as_str = val.to_str().expect("existing trigger is ascii");
        let new_header = format!("{as_str}, {event_name}");
        headers.insert(
            "Hx-Trigger",
            HeaderValue::from_str(&new_header)
                .expect("event name is a valid string"),
        );
    } else {
        headers.insert(
            "Hx-Trigger",
            HeaderValue::from_str(event_name)
                .expect("event name is a valid string"),
        );
    }

    headers
}

/// Insert the Hx-Trugger header into a HeaderMap such that the table reload
/// will occur on the collection view.
fn reload_table(headers: HeaderMap) -> HeaderMap {
    trigger_event(headers, "reload-pages")
}

fn reload_add_filter_button(headers: HeaderMap) -> HeaderMap {
    trigger_event(headers, "reload-add-filter-button")
}

#[derive(Debug, Deserialize)]
pub struct BoolForm {
    value: String,
}
pub async fn handle_bool_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<BoolForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterBool::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let new_filter = if form.value == "true" {
        models::FilterBool {
            id: filter.id,
            r#type: models::FilterType::Eq("Exactly Equals".into()),
            prop_id: filter.prop_id,
            value: true,
        }
    } else if form.value == "false" {
        models::FilterBool {
            id: filter.id,
            r#type: models::FilterType::Eq("Exactly Equals".into()),
            prop_id: filter.prop_id,
            value: false,
        }
    } else if form.value == "is-empty" {
        models::FilterBool {
            id: filter.id,
            r#type: models::FilterType::IsEmpty("Is Empty".into()),
            prop_id: filter.prop_id,
            value: true,
        }
    } else {
        return Ok((
            StatusCode::BAD_REQUEST,
            headers,
            "Invalid value".to_string(),
        ));
    };

    if new_filter != filter {
        new_filter.save(&db).await?;
        headers = reload_table(headers);
    };
    let has_capacity =
        db_ops::does_collection_have_capacity_for_additional_filters(
            &db,
            related_prop.collection_id,
        )
        .await?;
    let add_filter_button = if has_capacity {
        components::AddFilterButton {
            collection_id: related_prop.collection_id,
        }
        .render()
    } else {
        components::AddFilterButtonPlaceholder {
            collection_id: related_prop.collection_id,
        }
        .render()
    };
    Ok((
        StatusCode::OK,
        headers,
        [
            r#"<div class="flex flex-row gap-2">"#,
            &add_filter_button,
            &components::FilterBool {
                filter: &new_filter,
                prop_name: &related_prop.name,
            }
            .render(),
            r#"</div>"#,
        ]
        .join(""),
    ))
}

pub async fn get_int_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterInt::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterInt {
        filter,
        prop_name: &related_prop.name,
    }
    .render())
}

pub async fn get_int_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterInt::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::IntFilterForm {
        filter,
        prop_name: &related_prop.name,
    }
    .render())
}

#[derive(Deserialize)]
pub struct IntForm {
    pub value: i64,
    pub r#type: i32,
}
pub async fn handle_int_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<IntForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterInt::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let form_type = models::FilterType::new(form.r#type, "".into());
    let new_filter = models::FilterInt {
        id: filter.id,
        prop_id: filter.prop_id,
        r#type: form_type,
        value: form.value,
    };
    if new_filter != filter {
        new_filter.save(&db).await?;
        headers = reload_table(headers);
    };
    let has_capacity =
        db_ops::does_collection_have_capacity_for_additional_filters(
            &db,
            related_prop.collection_id,
        )
        .await?;
    let add_filter_button = if has_capacity {
        components::AddFilterButton {
            collection_id: related_prop.collection_id,
        }
        .render()
    } else {
        components::AddFilterButtonPlaceholder {
            collection_id: related_prop.collection_id,
        }
        .render()
    };
    Ok((
        headers,
        [
            r#"<div class="flex flex-row gap-2">"#,
            &add_filter_button,
            &components::FilterInt {
                filter: &new_filter,
                prop_name: &related_prop.name,
            }
            .render(),
            r#"</div>"#,
        ]
        .join(""),
    ))
}

pub async fn get_int_rng_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterIntRng::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterIntRng {
        filter,
        prop_name: &related_prop.name,
    }
    .render())
}

pub async fn get_int_rng_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterIntRng::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::IntRngFilterForm {
        filter,
        prop_name: &related_prop.name,
    }
    .render())
}

#[derive(Deserialize)]
pub struct IntRngForm {
    pub start: i64,
    pub end: i64,
    pub r#type: i32,
}
pub async fn handle_int_rng_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<IntRngForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterIntRng::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let form_type = models::FilterType::new(form.r#type, "".into());
    let new_filter = models::FilterIntRng {
        r#type: form_type.clone(),
        start: form.start,
        end: form.end,
        ..filter
    };
    if new_filter != filter {
        new_filter.save(&db).await?;
        headers = reload_table(headers);
    };
    let has_capacity =
        db_ops::does_collection_have_capacity_for_additional_filters(
            &db,
            related_prop.collection_id,
        )
        .await?;
    let add_filter_button = if has_capacity {
        components::AddFilterButton {
            collection_id: related_prop.collection_id,
        }
        .render()
    } else {
        components::AddFilterButtonPlaceholder {
            collection_id: related_prop.collection_id,
        }
        .render()
    };
    Ok((
        headers,
        [
            r#"<div class="flex flex-row gap-2">"#,
            &add_filter_button,
            &components::FilterIntRng {
                filter: &new_filter,
                prop_name: &related_prop.name,
            }
            .render(),
            r#"</div>"#,
        ]
        .join(""),
    ))
}

pub async fn choose_prop_for_filter(
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let props = models::Prop::list(
        &db,
        &db_ops::ListPropQuery {
            collection_id: Some(collection_id),
            order_in: None,
            exact_ids: None,
        },
    )
    .await?;
    let (fb, fi, fir) = db_ops::get_filters(&db, collection_id).await?;
    let mut props_with_filter = HashSet::new();
    for f in fb {
        props_with_filter.insert(f.prop_id);
    }
    for f in fi {
        props_with_filter.insert(f.prop_id);
    }
    for f in fir {
        props_with_filter.insert(f.prop_id);
    }

    let props: Vec<&models::Prop> = props
        .iter()
        .filter(|p| !props_with_filter.contains(&p.id))
        .collect();

    Ok(components::ChoosePropForFilter { props: &props }.render())
}

pub async fn new_filter_type_select(
    State(AppState { db }): State<AppState>,
    Path(prop_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: prop_id }).await?;
    let options = models::FilterType::get_supported_filter_types(prop.type_id);
    Ok(components::NewFilterTypeOptions {
        options: &options,
        prop_id,
        prop_type: &prop.type_id,
    }
    .render())
}

#[derive(Deserialize)]
pub struct NewFilterQuery {
    pub type_id: Option<i32>,
}

pub async fn create_new_bool_filter(
    State(AppState { db }): State<AppState>,
    Path(prop_id): Path<i32>,
    Query(NewFilterQuery { type_id }): Query<NewFilterQuery>,
) -> Result<impl IntoResponse, ServerError> {
    let r#type = if let Some(type_id) = type_id {
        models::FilterType::new(type_id, "".into())
    } else {
        models::FilterType::Eq("".into())
    };
    let query = db_ops::GetPropQuery { id: prop_id };
    let (prop, filter) = join!(
        models::Prop::get(&db, &query),
        db_ops::create_bool_filter(&db, prop_id, r#type)
    );
    let prop = prop?;
    let filter = filter?;

    Ok(components::BoolFilterForm {
        filter: &filter,
        prop_name: &prop.name,
    }
    .render())
}
pub async fn create_new_int_filter(
    State(AppState { db }): State<AppState>,
    Path(prop_id): Path<i32>,
    Query(NewFilterQuery { type_id }): Query<NewFilterQuery>,
) -> Result<impl IntoResponse, ServerError> {
    let r#type = if let Some(type_id) = type_id {
        models::FilterType::new(type_id, "".into())
    } else {
        models::FilterType::Eq("".into())
    };
    let query = db_ops::GetPropQuery { id: prop_id };
    let (prop, filter) = join!(
        models::Prop::get(&db, &query),
        db_ops::create_int_filter(&db, prop_id, r#type)
    );
    let prop = prop?;
    let filter = filter?;

    Ok(components::IntFilterForm {
        filter: &filter,
        prop_name: &prop.name,
    }
    .render())
}

pub async fn create_new_int_rng_filter(
    State(AppState { db }): State<AppState>,
    Path(prop_id): Path<i32>,
    Query(NewFilterQuery { type_id }): Query<NewFilterQuery>,
) -> Result<impl IntoResponse, ServerError> {
    let r#type = if let Some(type_id) = type_id {
        models::FilterType::new(type_id, "".into())
    } else {
        models::FilterType::Eq("".into())
    };
    let query = db_ops::GetPropQuery { id: prop_id };
    let (prop, filter) = join!(
        models::Prop::get(&db, &query),
        db_ops::create_int_rng_filter(&db, prop_id, r#type)
    );
    let prop = prop?;
    let filter = filter?;

    Ok(components::IntRngFilterForm {
        filter: &filter,
        prop_name: &prop.name,
    }
    .render())
}

/// I pulled this out into a separate request because it requires its own
/// database query. We only want to show the filter button if there are
/// props in the workspace that do not have any filters already.
pub async fn get_add_filter_button(
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let does_it_tho =
        db_ops::does_collection_have_capacity_for_additional_filters(
            &db,
            collection_id,
        )
        .await?;

    if does_it_tho {
        Ok(components::AddFilterButton { collection_id }.render())
    } else {
        Ok(components::AddFilterButtonPlaceholder { collection_id }.render())
    }
}

pub async fn delete_bool_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterBool::get(&db, &db_ops::GetFilterQuery { id }).await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}

pub async fn delete_int_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterInt::get(&db, &db_ops::GetFilterQuery { id }).await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}

pub async fn delete_int_rng_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterIntRng::get(&db, &db_ops::GetFilterQuery { id }).await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}

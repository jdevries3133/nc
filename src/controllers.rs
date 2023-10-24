use super::{
    auth, components,
    components::{Component, FilterUi},
    db_ops,
    db_ops::{DbModel, FilterDb},
    errors::ServerError,
    htmx, models,
    models::{AppState, PropVal},
    pw,
    routes::Route,
    session,
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

pub async fn root() -> impl IntoResponse {
    components::Page {
        title: "NC!",
        children: Box::new(components::Home {}),
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

/// You may be wondering why this sits on a separate response while the
/// tailwind styles are inlined into the page template and basically
/// hard-coded into every initial response. This is because the CSS is a
/// blocker for page rendering, so we want it right there in the initial
/// response. Meanwhile, it's fine for the browser to fetch and run HTMX
/// asynchronously since it doesn't really need to be on the page until the
/// first user interaction.
///
/// Additionally, our HTMX version does not change very often. We can exploit
/// browser cachine to mostly never need to serve this resource, making the
/// app more responsive and cutting down on overall bandwidth. That's also why
/// we have the HTMX version in the URL path -- because we need to bust the
/// browser cache every time we upgrade.
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
            title: &format!("Workspace ({name})"),
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
    let (pages, props) =
        db_ops::list_pages(&db, collection_id, page.unwrap_or(0)).await?;

    Ok(components::PageList {
        pages: &pages,
        props: &props,
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
            title: &format!("Set Prop Order (collection {})", collection_id),
            children: Box::new(components::PropOrderForm { props }),
        }
        .render()
    })
}

pub async fn new_bool_propval_form(
    Path((page_id, prop_id)): Path<(i32, i32)>,
) -> impl IntoResponse {
    models::PvBool {
        page_id,
        prop_id,
        value: Some(false),
    }
    .render()
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
    if Some(new_val) != pvb.value {
        pvb.value = Some(value.is_some());
        pvb.save(&db).await?;
    }

    Ok(pvb.render())
}

pub async fn new_int_propval_form(
    Path((page_id, prop_id)): Path<(i32, i32)>,
) -> impl IntoResponse {
    models::PvInt {
        page_id,
        prop_id,
        value: Some(0),
    }
    .render()
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
        if Some(v) != existing.value {
            existing.value = Some(v);
            existing.save(&db).await?;
        }
    };
    Ok(existing.render())
}

pub async fn new_float_propval_form(
    Path((page_id, prop_id)): Path<(i32, i32)>,
) -> impl IntoResponse {
    models::PvFloat {
        page_id,
        prop_id,
        value: Some(0.0),
    }
    .render()
}

pub async fn new_date_propval_form(
    Path((page_id, prop_id)): Path<(i32, i32)>,
) -> impl IntoResponse {
    models::PvDate {
        page_id,
        prop_id,
        value: Some(chrono::Local::now().date_naive()),
    }
    .render()
}

#[derive(Deserialize)]
pub struct PvFloatForm {
    value: Option<f64>,
}
pub async fn save_pv_float(
    State(AppState { db }): State<AppState>,
    Path((page_id, prop_id)): Path<(i32, i32)>,
    Form(PvFloatForm { value }): Form<PvFloatForm>,
) -> Result<impl IntoResponse, ServerError> {
    let mut existing = models::PvFloat::get_or_init(
        &db,
        &db_ops::PvGetQuery { prop_id, page_id },
    )
    .await;

    if let Some(v) = value {
        if Some(v) != existing.value {
            existing.value = Some(v);
            existing.save(&db).await?;
        }
    };
    Ok(existing.render())
}

#[derive(Deserialize)]
pub struct PvDateForm {
    value: Option<chrono::NaiveDate>,
}
pub async fn save_pv_date(
    State(AppState { db }): State<AppState>,
    Path((page_id, prop_id)): Path<(i32, i32)>,
    Form(PvDateForm { value }): Form<PvDateForm>,
) -> Result<impl IntoResponse, ServerError> {
    let mut existing = models::PvDate::get_or_init(
        &db,
        &db_ops::PvGetQuery { prop_id, page_id },
    )
    .await;

    if let Some(v) = value {
        if Some(v) != existing.value {
            existing.value = Some(v);
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
            title: "New Page",
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
            title: &page.title,
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
    id: Option<i32>,
    title: String,
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
    let headers = HeaderMap::new();
    let collection_route = Route::Collection(Some(collection_id));
    Ok((
        axum::http::StatusCode::CREATED,
        htmx::redirect(headers, &collection_route.as_string()),
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
    let (
        bool_filters,
        int_filters,
        int_rng_filters,
        float_filters,
        float_rng_filters,
        date_filters,
        date_rng_filters,
    ) = db_ops::get_filters(&db, collection_id).await?;
    if bool_filters.is_empty()
        && int_filters.is_empty()
        && int_rng_filters.is_empty()
        && float_filters.is_empty()
        && float_rng_filters.is_empty()
        && date_filters.is_empty()
        && date_rng_filters.is_empty()
    {
        let hide_route =
            Route::CollectionHideFilterToolbar(Some(collection_id));
        return Ok(components::Div {
            class: "my-2",
            hx_trigger: Some("toggle-filter-toolbar from:body"),
            hx_get: Some(&hide_route.as_string()),
            children: Box::new(components::AddFilterButton { collection_id }),
        }
        .render());
    };
    // I realize that int / int-rng and float / float-rng can introduce
    // duplicates into this vector and I should use a HashSet, though a
    // duplicate in a WHERE IN clause won't kill anyone (I hope)
    let mut all_props: Vec<i32> = Vec::with_capacity(
        bool_filters.len()
            + int_filters.len()
            + int_rng_filters.len()
            + float_filters.len()
            + float_rng_filters.len()
            + date_filters.len()
            + date_rng_filters.len(),
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
    for f in &float_filters[..] {
        all_props.push(f.prop_id);
    }
    for f in &float_rng_filters[..] {
        all_props.push(f.prop_id);
    }
    for f in &date_filters[..] {
        all_props.push(f.prop_id);
    }
    for f in &date_filters[..] {
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
        float_filters,
        float_rng_filters,
        date_filters,
        date_rng_filters,
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
    .render_chip())
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

    Ok(components::FilterBool {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
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

    Ok((
        StatusCode::OK,
        headers,
        components::FilterBool {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
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
    .render_chip())
}

pub async fn get_float_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterFloat::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterFloat {
        filter,
        prop_name: &related_prop.name,
    }
    .render_chip())
}

pub async fn get_date_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterDate::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterDate {
        filter,
        prop_name: &related_prop.name,
    }
    .render_chip())
}

pub async fn get_date_rng_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterDateRng::get(&db, &db_ops::GetFilterQuery { id })
            .await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterDateRng {
        filter,
        prop_name: &related_prop.name,
    }
    .render_chip())
}

pub async fn get_float_rng_filter_chip(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterFloatRng::get(&db, &db_ops::GetFilterQuery { id })
            .await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterFloatRng {
        filter,
        prop_name: &related_prop.name,
    }
    .render_chip())
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

    Ok(components::FilterInt {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
}

pub async fn get_float_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterFloat::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterFloat {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
}

pub async fn get_float_rng_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterFloatRng::get(&db, &db_ops::GetFilterQuery { id })
            .await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterFloatRng {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
}

pub async fn get_date_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterDate::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterDate {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
}

pub async fn get_date_rng_filter_form(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        &models::FilterDateRng::get(&db, &db_ops::GetFilterQuery { id })
            .await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;

    Ok(components::FilterDateRng {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
}

#[derive(Deserialize)]
pub struct IntForm {
    value: i64,
    r#type: i32,
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

    Ok((
        headers,
        components::FilterInt {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
    ))
}

#[derive(Deserialize)]
pub struct FloatForm {
    value: f64,
    r#type: i32,
}
pub async fn handle_float_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<FloatForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterFloat::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let form_type = models::FilterType::new(form.r#type, "".into());
    let new_filter = models::FilterFloat {
        id: filter.id,
        prop_id: filter.prop_id,
        r#type: form_type,
        value: form.value,
    };
    new_filter.save(&db).await?;
    headers = reload_table(headers);

    Ok((
        headers,
        components::FilterFloat {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
    ))
}

#[derive(Deserialize)]
pub struct DateForm {
    value: chrono::NaiveDate,
    r#type: i32,
}
pub async fn handle_date_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<DateForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterDate::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let form_type = models::FilterType::new(form.r#type, "".into());
    let new_filter = models::FilterDate {
        id: filter.id,
        prop_id: filter.prop_id,
        r#type: form_type,
        value: form.value,
    };
    new_filter.save(&db).await?;
    headers = reload_table(headers);

    Ok((
        headers,
        components::FilterDate {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
    ))
}

#[derive(Deserialize)]
pub struct FloatRngForm {
    start: f64,
    end: f64,
    r#type: i32,
}
pub async fn handle_float_rng_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<FloatRngForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterFloatRng::get(&db, &db_ops::GetFilterQuery { id })
            .await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let form_type = models::FilterType::new(form.r#type, "".into());
    let new_filter = models::FilterFloatRng {
        id: filter.id,
        prop_id: filter.prop_id,
        r#type: form_type,
        start: form.start,
        end: form.end,
    };
    new_filter.save(&db).await?;
    headers = reload_table(headers);

    Ok((
        headers,
        components::FilterFloatRng {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
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
    .render_chip())
}

#[derive(Deserialize)]
pub struct DateRngForm {
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    r#type: i32,
}
pub async fn handle_date_rng_form_submit(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<DateRngForm>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterDateRng::get(&db, &db_ops::GetFilterQuery { id }).await?;
    let related_prop =
        models::Prop::get(&db, &db_ops::GetPropQuery { id: filter.prop_id })
            .await?;
    let mut headers = HeaderMap::new();
    let form_type = models::FilterType::new(form.r#type, "".into());
    let new_filter = models::FilterDateRng {
        id: filter.id,
        prop_id: filter.prop_id,
        r#type: form_type,
        start: form.start,
        end: form.end,
    };
    new_filter.save(&db).await?;
    headers = reload_table(headers);

    Ok((
        headers,
        components::FilterDateRng {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
    ))
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

    Ok(components::FilterIntRng {
        filter,
        prop_name: &related_prop.name,
    }
    .render_form())
}

#[derive(Deserialize)]
pub struct IntRngForm {
    start: i64,
    end: i64,
    r#type: i32,
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

    Ok((
        headers,
        components::FilterIntRng {
            filter: &new_filter,
            prop_name: &related_prop.name,
        }
        .render_chip(),
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
    let (fb, fi, fir, ffl, fflr, fd, fdr) =
        db_ops::get_filters(&db, collection_id).await?;
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
    for f in ffl {
        props_with_filter.insert(f.prop_id);
    }
    for f in fflr {
        props_with_filter.insert(f.prop_id);
    }
    for f in fd {
        props_with_filter.insert(f.prop_id);
    }
    for f in fdr {
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
    type_id: Option<i32>,
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
        models::FilterBool::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
            &components::FilterBool {
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            "</div>",
        ]
        .join(""),
    ))
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
        models::FilterInt::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            "</div>",
        ]
        .join(""),
    ))
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
        models::FilterIntRng::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            r#"</div>"#,
        ]
        .join(""),
    ))
}

pub async fn create_new_float_filter(
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
        models::FilterFloat::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
            &components::FilterFloat {
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            "</div>",
        ]
        .join(""),
    ))
}

pub async fn create_new_float_rng_filter(
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
        models::FilterFloatRng::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
            &components::FilterFloatRng {
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            r#"</div>"#,
        ]
        .join(""),
    ))
}

pub async fn create_new_date_filter(
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
        models::FilterDate::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
            &components::FilterDate {
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            "</div>",
        ]
        .join(""),
    ))
}

pub async fn create_new_date_rng_filter(
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
        models::FilterDateRng::create(&db, prop_id, r#type)
    );
    let related_prop = prop?;
    let filter = filter?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);

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
            &components::FilterDateRng {
                filter: &filter,
                prop_name: &related_prop.name,
            }
            .render_form(),
            r#"</div>"#,
        ]
        .join(""),
    ))
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

pub async fn delete_float_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterFloat::get(&db, &db_ops::GetFilterQuery { id }).await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}

pub async fn delete_float_rng_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterFloatRng::get(&db, &db_ops::GetFilterQuery { id })
            .await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}

pub async fn delete_date_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterDate::get(&db, &db_ops::GetFilterQuery { id }).await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}

pub async fn delete_date_rng_filter(
    State(AppState { db }): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ServerError> {
    let filter =
        models::FilterDateRng::get(&db, &db_ops::GetFilterQuery { id }).await?;
    filter.delete(&db).await?;

    let headers = HeaderMap::new();
    let headers = reload_table(headers);
    let headers = reload_add_filter_button(headers);

    Ok((headers, ""))
}
pub async fn show_sort_toolbar(
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
    if let Ok(sort) = models::CollectionSort::get(
        &db,
        &db_ops::GetSortQuery { collection_id },
    )
    .await
    {
        Ok(components::SortToolbar {
            collection_id,
            prop_choices: &props[..],
            sort_type: sort.r#type,
            default_selected_prop: sort.prop_id,
        }
        .render())
    } else {
        Ok(components::SortToolbar {
            collection_id,
            prop_choices: &props[..],
            sort_type: Some(models::SortType::Asc),
            default_selected_prop: None,
        }
        .render())
    }
}

pub async fn hide_sort_toolbar(
    Path(collection_id): Path<i32>,
) -> impl IntoResponse {
    components::SortToolbarPlaceholder { collection_id }.render()
}

#[derive(Debug, Deserialize)]
pub struct SortForm {
    sort_by: i32,
    sort_order: i32,
}

pub async fn handle_sort_form_submit(
    State(AppState { db }): State<AppState>,
    Path(collection_id): Path<i32>,
    Form(form): Form<SortForm>,
) -> Result<impl IntoResponse, ServerError> {
    // I'm being a bad person and using -1 as a sentinel for NULL.
    //
    // Don't @ me
    let new_sort = if form.sort_by == -1 {
        models::CollectionSort {
            collection_id,
            prop_id: None,
            r#type: None,
        }
    } else {
        models::CollectionSort {
            collection_id,
            prop_id: Some(form.sort_by),
            r#type: Some(models::SortType::from_int(form.sort_order)?),
        }
    };
    // Implicitly treating 'error' as 'does not exist'
    let existing_sort = if let Ok(sort) = models::CollectionSort::get(
        &db,
        &db_ops::GetSortQuery { collection_id },
    )
    .await
    {
        Some(sort)
    } else {
        None
    };
    let headers = HeaderMap::new();
    Ok(
        if existing_sort.is_none() || new_sort != existing_sort.unwrap() {
            new_sort.save(&db).await?;
            let headers = reload_table(headers);
            (
                headers,
                components::SortOrderSavedConfirmation { collection_id }
                    .render(),
            )
        } else {
            (
                headers,
                components::SortOrderSavedConfirmation { collection_id }
                    .render(),
            )
        },
    )
}

pub async fn get_registration_form(headers: HeaderMap) -> impl IntoResponse {
    let form = components::RegisterForm {};

    if headers.contains_key("Hx-Request") {
        form.render()
    } else {
        components::Page {
            title: "Register",
            children: Box::new(form),
        }
        .render()
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    username: String,
    email: String,
    password: String,
    secret_word: String,
}

pub async fn handle_registration(
    State(AppState { db }): State<AppState>,
    Form(form): Form<RegisterForm>,
) -> Result<impl IntoResponse, ServerError> {
    let headers = HeaderMap::new();
    if form.secret_word.to_lowercase() != "blorp" {
        let register_route = Route::Register;
        return Ok((
            headers,
            format!(
                r#"<p hx-trigger="load delay:1s" hx-get="{register_route}">Nice try ya chungus</p>"#
            ),
        ));
    };
    let hashed_pw = pw::hash_new(&form.password);
    let user =
        db_ops::create_user(&db, form.username, form.email, &hashed_pw).await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let session = session::Session {
        user,
        created_at: now,
    };
    let headers = session.update_headers(headers);
    let headers = htmx::redirect(headers, "/collection/1");
    Ok((headers, "OK".to_string()))
}

pub async fn get_login_form(headers: HeaderMap) -> impl IntoResponse {
    let form = components::LoginForm {};

    if headers.contains_key("Hx-Request") {
        form.render()
    } else {
        components::Page {
            title: "Login",
            children: Box::new(form),
        }
        .render()
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    /// Username or email
    identifier: String,
    password: String,
}

pub async fn handle_login(
    State(AppState { db }): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, ServerError> {
    let session =
        auth::authenticate(&db, &form.identifier, &form.password).await;
    let headers = HeaderMap::new();
    if let Ok(session) = session {
        let headers = session.update_headers(headers);
        let headers = htmx::redirect(headers, "/collection/1");
        Ok((headers, "OK".to_string()))
    } else {
        let login_route = Route::Login;
        Ok((
            headers,
            format!(
                r#"<p hx-trigger="load delay:1s" hx-get="{login_route}">Nice try ya chungus</p>"#
            ),
        ))
    }
}

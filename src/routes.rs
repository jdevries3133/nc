use super::{controllers, models};
use axum::routing::{delete, get, post, Router};

/// This enum contains all of the route strings in the application. This
/// solves several problems:
///
/// 1. Maintaining a single source of truth for route paths, even if it has
///    multiple controllers for various HTTP methods
/// 2. Making it easier to refactor routing without needing to keep the axum
///    router and paths referenced in routers in sync.
/// 3. Making it easier to jump from a component to the handlers in a route
///    it references and visa versa.
/// 4. Further decoupling the app from the underlying HTTP.
/// 5. Allowing documentation on a route, which is super useful for quick
///    reference when authoring components.
///
/// For each route, the parameters are inside an Option<T>. If no parameters
/// are provided, we'll construct the route with the `:id` template in it
/// for the Axum router.
pub enum Route {
    Collection(Option<i32>),
    CollectionPageSubmission(Option<i32>),
    CollectionNewPageForm(Option<i32>),
    CollectionListPages(Option<i32>),
    CollectionChangePropOrder(Option<i32>),
    CollectionIncrementPropOrder(Option<(i32, i32)>),
    CollectionDecrementPropOrder(Option<(i32, i32)>),
    CollectionShowFilterToolbar(Option<i32>),
    CollectionHideFilterToolbar(Option<i32>),
    CollectionChoosePropForFilter(Option<i32>),
    CollectionAddFilterButton(Option<i32>),
    CollectionShowSortToolbar(Option<i32>),
    CollectionHideSortToolbar(Option<i32>),
    CollectionSort(Option<i32>),
    PropNewFilterTypeSelect(Option<i32>),
    PropNewBoolFilter(Option<i32>),
    PropNewIntFilter(Option<i32>),
    PropNewIntRngFilter(Option<i32>),
    PropNewFloatFilter(Option<i32>),
    PropNewFloatRngFilter(Option<i32>),
    FilterBoolChip(Option<i32>),
    /// Has GET (returning a form), POST (accepting submission), and DELETE
    FilterBool(Option<i32>),
    FilterIntChip(Option<i32>),
    /// Has GET (returning a form), POST (accepting submission), and DELETE
    FilterInt(Option<i32>),
    FilterIntRngChip(Option<i32>),
    /// Has GET (returning a form), POST (accepting submission), and DELETE
    FilterIntRng(Option<i32>),
    FilterFloatChip(Option<i32>),
    FilterFloat(Option<i32>),
    FilterFloatRngChip(Option<i32>),
    /// Has GET (returning a form), POST (accepting submission), and DELETE
    FilterFloatRng(Option<i32>),
    Page(Option<i32>),
    PageSubmit,
    PageContent(Option<i32>),
    PageBoolProp(Option<(i32, i32)>),
    PageIntProp(Option<(i32, i32)>),
    PageFloatProp(Option<(i32, i32)>),
    PageNewBoolProp(Option<(i32, i32)>),
    PageNewIntProp(Option<(i32, i32)>),
    PageNewFloatProp(Option<(i32, i32)>),
    Root,
    Ping,
    Register,
    Login,
    /// The static content route where HTMX javascript library is served, which
    /// we are vendoring.
    Htmx,
}

impl Route {
    pub fn as_string(&self) -> String {
        match self {
            Self::Collection(params) => match params {
                Some(id) => format!("/collection/{id}"),
                None => "/collection/:id".into(),
            },
            Self::CollectionPageSubmission(params) => match params {
                Some(id) => format!("/collection/{id}"),
                None => "/collection/:id".into(),
            },
            Self::CollectionNewPageForm(params) => match params {
                Some(id) => format!("/collection/{id}/new-page"),
                None => "/collection/:id/new-page".into(),
            },
            Self::CollectionListPages(params) => match params {
                Some(id) => format!("/collection/{id}/list-pages"),
                None => "/collection/:id/list-pages".into(),
            },
            Self::CollectionChangePropOrder(params) => match params {
                Some(id) => format!("/collection/{id}/prop-order"),
                None => "/collection/:id/prop-order".into(),
            },
            Self::CollectionIncrementPropOrder(params) => match params {
                Some((collection_id, prop_id)) => {
                    format!("/collection/{collection_id}/prop/{prop_id}/up")
                }
                None => "/collection/:collection_id/prop/:prop_id/up".into(),
            },
            Self::CollectionDecrementPropOrder(params) => match params {
                Some((collection_id, prop_id)) => {
                    format!("/collection/{collection_id}/prop/{prop_id}/down")
                }
                None => "/collection/:collection_id/prop/:prop_id/down".into(),
            },
            Route::CollectionShowFilterToolbar(params) => match params {
                Some(id) => format!("/collection/{id}/show-filter-toolbar"),
                None => "/collection/:id/show-filter-toolbar".into(),
            },
            Self::CollectionHideFilterToolbar(params) => match params {
                Some(id) => format!("/collection/{id}/hide-sort-toolbar"),
                None => "/collection/:id/hide-sort-toolbar".into(),
            },
            Self::CollectionChoosePropForFilter(params) => match params {
                Some(id) => format!("/collection/{id}/choose-prop-for-filter"),
                None => "/collection/:id/choose-prop-for-filter".into(),
            },
            Self::CollectionAddFilterButton(params) => match params {
                Some(id) => format!("/collection/{id}/add-filter-button"),
                None => "/collection/:id/add-filter-button".into(),
            },
            Self::CollectionHideSortToolbar(params) => match params {
                Some(id) => format!("/collection/{id}/hide-filter-toolbar"),
                None => "/collection/:id/hide-filter-toolbar".into(),
            },
            Route::CollectionShowSortToolbar(params) => match params {
                Some(id) => format!("/collection/{id}/show-sort-toolbar"),
                None => "/collection/:id/show-sort-toolbar".into(),
            },
            Route::CollectionSort(params) => match params {
                Some(id) => format!("/collection/{id}/sort"),
                None => "/collection/:id/sort".into(),
            },
            Self::PropNewFilterTypeSelect(params) => match params {
                Some(id) => format!("/prop/{id}/new-filter-type-select"),
                None => "/prop/:id/new-filter-type-select".into(),
            },
            Self::PropNewBoolFilter(params) => match params {
                Some(id) => format!("/prop/{id}/new-bool-filter"),
                None => "/prop/:id/new-bool-filter".into(),
            },
            Self::PropNewIntFilter(params) => match params {
                Some(id) => format!("/prop/{id}/new-int-filter"),
                None => "/prop/:id/new-int-filter".into(),
            },
            Self::PropNewIntRngFilter(params) => match params {
                Some(id) => format!("/prop/{id}/new-int-rng-filter"),
                None => "/prop/:id/new-int-rng-filter".into(),
            },
            Self::PropNewFloatFilter(params) => match params {
                Some(id) => format!("/prop/{id}/new-float-filter"),
                None => "/prop/:id/new-float-filter".into(),
            },
            Self::PropNewFloatRngFilter(params) => match params {
                Some(id) => format!("/prop/{id}/new-float-rng-filter"),
                None => "/prop/:id/new-float-rng-filter".into(),
            },
            Self::FilterBoolChip(params) => match params {
                Some(id) => format!("/filter/bool/{id}/chip"),
                None => "/filter/bool/:id/chip".into(),
            },
            Self::FilterBool(params) => match params {
                Some(id) => format!("/filter/bool/{id}"),
                None => "/filter/bool/:id".into(),
            },
            Self::FilterIntChip(params) => match params {
                Some(id) => format!("/filter/int/{id}/chip"),
                None => "/filter/int/:id/chip".into(),
            },
            Self::FilterInt(params) => match params {
                Some(id) => format!("/filter/int/{id}"),
                None => "/filter/int/:id".into(),
            },
            Self::FilterIntRngChip(params) => match params {
                Some(id) => format!("/filter/int-rng/{id}/chip"),
                None => "/filter/int-rng/:id/chip".into(),
            },
            Self::FilterIntRng(params) => match params {
                Some(id) => format!("/filter/int-rng/{id}"),
                None => "/filter/int-rng/:id".into(),
            },
            Self::FilterFloatChip(params) => match params {
                Some(id) => format!("/filter/float/{id}/chip"),
                None => "/filter/float/:id/chip".into(),
            },
            Self::FilterFloat(params) => match params {
                Some(id) => format!("/filter/float/{id}"),
                None => "/filter/float/:id".into(),
            },
            Self::FilterFloatRngChip(params) => match params {
                Some(id) => format!("/filter/float-rng/{id}/chip"),
                None => "/filter/float-rng/:id/chip".into(),
            },
            Self::FilterFloatRng(params) => match params {
                Some(id) => format!("/filter/float-rng/{id}"),
                None => "/filter/float-rng/:id".into(),
            },
            Self::Page(params) => match params {
                Some(id) => format!("/page/{id}"),
                None => "/page/:page_id".into(),
            },
            Self::PageSubmit => "/page".into(),
            Self::PageContent(params) => match params {
                Some(id) => format!("/page/{id}/content"),
                None => "/page/:id/content".into(),
            },
            Self::PageBoolProp(params) => match params {
                Some((page_id, prop_id)) => {
                    format!("/page/{page_id}/prop/{prop_id}/bool")
                }
                None => "/page/:page_id/prop/:prop_id/bool".into(),
            },
            Self::PageIntProp(params) => match params {
                Some((page_id, prop_id)) => {
                    format!("/page/{page_id}/prop/{prop_id}/int")
                }
                None => "/page/:page_id/prop/:prop_id/int".into(),
            },
            Self::PageFloatProp(params) => match params {
                Some((page_id, prop_id)) => {
                    format!("/page/{page_id}/prop/{prop_id}/float")
                }
                None => "/page/:page_id/prop/:prop_id/float".into(),
            },
            Self::PageNewBoolProp(params) => match params {
                Some((page_id, prop_id)) => {
                    format!("/page/{page_id}/prop/{prop_id}/new-bool")
                }
                None => "/page/:page_id/prop/:prop_id/new-bool".into(),
            },
            Self::PageNewIntProp(params) => match params {
                Some((page_id, prop_id)) => {
                    format!("/page/{page_id}/prop/{prop_id}/new-int")
                }
                None => "/page/:page_id/prop/:prop_id/new-int".into(),
            },
            Self::PageNewFloatProp(params) => match params {
                Some((page_id, prop_id)) => {
                    format!("/page/{page_id}/prop/{prop_id}/new-float")
                }
                None => "/page/:page_id/prop/:prop_id/new-float".into(),
            },
            Self::Root => "/".into(),
            Self::Ping => "/ping".into(),
            Self::Register => "/authentication/register".into(),
            Self::Login => "/authentication/login".into(),
            Self::Htmx => "/static/htmx-1.9.6".into(),
        }
    }
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

pub fn get_protected_routes() -> Router<models::AppState> {
    Router::new()
        .route(
            &Route::Collection(None).as_string(),
            get(controllers::get_collection),
        )
        .route(
            &Route::CollectionPageSubmission(None).as_string(),
            post(controllers::handle_page_submission),
        )
        .route(
            &Route::CollectionNewPageForm(None).as_string(),
            get(controllers::new_page_form),
        )
        .route(
            &Route::CollectionListPages(None).as_string(),
            get(controllers::collection_pages),
        )
        .route(
            &Route::CollectionChangePropOrder(None).as_string(),
            get(controllers::collection_prop_order),
        )
        .route(
            &Route::CollectionIncrementPropOrder(None).as_string(),
            post(controllers::increment_prop_order),
        )
        .route(
            &Route::CollectionDecrementPropOrder(None).as_string(),
            post(controllers::decrement_prop_order),
        )
        .route(
            &Route::CollectionShowFilterToolbar(None).as_string(),
            get(controllers::get_filter_toolbar),
        )
        .route(
            &Route::CollectionHideFilterToolbar(None).as_string(),
            get(controllers::hide_filter_toolbar),
        )
        .route(
            &Route::CollectionChoosePropForFilter(None).as_string(),
            get(controllers::choose_prop_for_filter),
        )
        .route(
            &Route::CollectionAddFilterButton(None).as_string(),
            get(controllers::get_add_filter_button),
        )
        .route(
            &Route::CollectionShowSortToolbar(None).as_string(),
            get(controllers::show_sort_toolbar),
        )
        .route(
            &Route::CollectionHideSortToolbar(None).as_string(),
            get(controllers::hide_sort_toolbar),
        )
        .route(
            &Route::CollectionSort(None).as_string(),
            post(controllers::handle_sort_form_submit),
        )
        .route(
            &Route::PropNewFilterTypeSelect(None).as_string(),
            get(controllers::new_filter_type_select),
        )
        .route(
            &Route::PropNewBoolFilter(None).as_string(),
            post(controllers::create_new_bool_filter),
        )
        .route(
            &Route::PropNewIntFilter(None).as_string(),
            post(controllers::create_new_int_filter),
        )
        .route(
            &Route::PropNewIntRngFilter(None).as_string(),
            post(controllers::create_new_int_rng_filter),
        )
        .route(
            &Route::PropNewFloatFilter(None).as_string(),
            post(controllers::create_new_float_filter),
        )
        .route(
            &Route::PropNewFloatRngFilter(None).as_string(),
            post(controllers::create_new_float_rng_filter),
        )
        .route(
            &Route::FilterBoolChip(None).as_string(),
            get(controllers::get_bool_filter_chip),
        )
        .route(
            &Route::FilterBool(None).as_string(),
            get(controllers::get_bool_filter_form),
        )
        .route(
            &Route::FilterBool(None).as_string(),
            post(controllers::handle_bool_form_submit),
        )
        .route(
            &Route::FilterBool(None).as_string(),
            delete(controllers::delete_bool_filter),
        )
        .route(
            &Route::FilterIntChip(None).as_string(),
            get(controllers::get_int_filter_chip),
        )
        .route(
            &Route::FilterInt(None).as_string(),
            get(controllers::get_int_filter_form),
        )
        .route(
            &Route::FilterInt(None).as_string(),
            post(controllers::handle_int_form_submit),
        )
        .route(
            &Route::FilterInt(None).as_string(),
            delete(controllers::delete_int_filter),
        )
        .route(
            &Route::FilterIntRngChip(None).as_string(),
            get(controllers::get_int_rng_filter_chip),
        )
        .route(
            &Route::FilterIntRng(None).as_string(),
            get(controllers::get_int_rng_filter_form),
        )
        .route(
            &Route::FilterIntRng(None).as_string(),
            post(controllers::handle_int_rng_form_submit),
        )
        .route(
            &Route::FilterIntRng(None).as_string(),
            delete(controllers::delete_int_rng_filter),
        )
        .route(
            &Route::FilterFloat(None).as_string(),
            get(controllers::get_float_filter_form),
        )
        .route(
            &Route::FilterFloat(None).as_string(),
            post(controllers::handle_float_form_submit),
        )
        .route(
            &Route::FilterFloat(None).as_string(),
            delete(controllers::delete_float_filter),
        )
        .route(
            &Route::FilterFloatChip(None).as_string(),
            get(controllers::get_float_filter_chip),
        )
        .route(
            &Route::FilterFloatRng(None).as_string(),
            get(controllers::get_float_rng_filter_form),
        )
        .route(
            &Route::FilterFloatRng(None).as_string(),
            post(controllers::handle_float_rng_form_submit),
        )
        .route(
            &Route::FilterFloatRng(None).as_string(),
            delete(controllers::delete_float_rng_filter),
        )
        .route(
            &Route::FilterFloatRngChip(None).as_string(),
            get(controllers::get_float_rng_filter_chip),
        )
        .route(
            &Route::Page(None).as_string(),
            get(controllers::existing_page_form),
        )
        .route(
            &Route::PageSubmit.as_string(),
            post(controllers::save_existing_page_form),
        )
        .route(
            &Route::PageContent(None).as_string(),
            get(controllers::get_content_form),
        )
        .route(
            &Route::PageContent(None).as_string(),
            post(controllers::handle_content_submission),
        )
        .route(
            &Route::PageBoolProp(None).as_string(),
            post(controllers::save_pv_bool),
        )
        .route(
            &Route::PageIntProp(None).as_string(),
            post(controllers::save_pv_int),
        )
        .route(
            &Route::PageFloatProp(None).as_string(),
            post(controllers::save_pv_float),
        )
        .route(
            &Route::PageNewBoolProp(None).as_string(),
            get(controllers::new_bool_propval_form),
        )
        .route(
            &Route::PageNewIntProp(None).as_string(),
            get(controllers::new_int_propval_form),
        )
        .route(
            &Route::PageNewFloatProp(None).as_string(),
            get(controllers::new_float_propval_form),
        )
}

pub fn get_public_routes() -> Router<models::AppState> {
    Router::new()
        .route(&Route::Root.as_string(), get(controllers::root))
        .route(&Route::Ping.as_string(), get(controllers::pong))
        .route(
            &Route::Register.as_string(),
            get(controllers::get_registration_form),
        )
        .route(
            &Route::Register.as_string(),
            post(controllers::handle_registration),
        )
        .route(&Route::Login.as_string(), get(controllers::get_login_form))
        .route(&Route::Login.as_string(), post(controllers::handle_login))
        .route(&Route::Htmx.as_string(), get(controllers::get_htmx_js))
}

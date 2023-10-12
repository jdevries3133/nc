use super::{controllers, models};
use axum::routing::{delete, get, post, Router};

#[rustfmt::skip]
pub fn get_routes() -> Router<models::AppState> {
    Router::new()
        .route("/",                                                 get(controllers::root))
        .route("/ping",                                             get(controllers::pong))
        .route("/collection/:id",                                   get(controllers::get_collection))
        .route("/collection/:id",                                   post(controllers::handle_page_submission))
        .route("/collection/:id/new-page",                          get(controllers::new_page_form))
        .route("/collection/:id/list-pages",                        get(controllers::collection_pages))
        .route("/collection/:id/prop-order",                        get(controllers::collection_prop_order))
        .route("/collection/:id/prop/:id/up",                       post(controllers::increment_prop_order))
        .route("/collection/:id/prop/:id/down",                     post(controllers::decrement_prop_order))
        .route("/collection/:id/show-filter-toolbar",               get(controllers::get_filter_toolbar))
        .route("/collection/:id/hide-filter-toolbar",               get(controllers::hide_filter_toolbar))
        .route("/collection/:id/choose-prop-for-filter",            get(controllers::choose_prop_for_filter))
        .route("/collection/:id/add-filter-button",                 get(controllers::get_add_filter_button))
        .route("/collection/:id/show-sort-toolbar",                 get(controllers::show_sort_toolbar))
        .route("/collection/:id/hide-sort-toolbar",                 get(controllers::hide_sort_toolbar))
        .route("/collection/:id/sort",                              post(controllers::handle_sort_form_submit))
        .route("/prop/:id/new-filter-type-select",                  get(controllers::new_filter_type_select))
        .route("/prop/:id/new-bool-filter",                         post(controllers::create_new_bool_filter))
        .route("/prop/:id/new-int-filter",                          post(controllers::create_new_int_filter))
        .route("/prop/:id/new-int-rng-filter",                      post(controllers::create_new_int_rng_filter))
        .route("/filter/bool/:id/chip",                             get(controllers::get_bool_filter_chip))
        .route("/filter/bool/:id",                                  get(controllers::get_bool_filter_form))
        .route("/filter/bool/:id",                                  post(controllers::handle_bool_form_submit))
        .route("/filter/bool/:id",                                  delete(controllers::delete_bool_filter))
        .route("/filter/int/:id/chip",                              get(controllers::get_int_filter_chip))
        .route("/filter/int/:id",                                   get(controllers::get_int_filter_form))
        .route("/filter/int/:id",                                   post(controllers::handle_int_form_submit))
        .route("/filter/int/:id",                                   delete(controllers::delete_int_filter))
        .route("/filter/int-rng/:id/chip",                          get(controllers::get_int_rng_filter_chip))
        .route("/filter/int-rng/:id",                               get(controllers::get_int_rng_filter_form))
        .route("/filter/int-rng/:id",                               post(controllers::handle_int_rng_form_submit))
        .route("/filter/int-rng/:id",                               delete(controllers::delete_int_rng_filter))
        .route("/page/:page_id",                                    get(controllers::existing_page_form))
        .route("/page",                                             post(controllers::save_existing_page_form))
        .route("/page/:page_id/content",                            get(controllers::get_content_form))
        .route("/page/:page_id/content",                            post(controllers::handle_content_submission))
        .route("/page/:page_id/prop/:prop_id/bool",                 post(controllers::save_pv_bool))
        .route("/page/:page_id/prop/:prop_id/int",                  post(controllers::save_pv_int))
        .route("/page/:page_id/prop/:prop_id/new-bool",             get(controllers::new_bool_propval_form))
        .route("/page/:page_id/prop/:prop_id/new-int",              get(controllers::new_int_propval_form))
        .route("/static/htmx-1.9.6",                                get(controllers::get_htmx_js))
}

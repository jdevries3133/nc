use super::{controllers, models};
use axum::routing::{delete, get, post, Router};

#[rustfmt::skip]
pub fn get_routes() -> Router<models::AppState> {
    Router::new()
        .route("/",                                 get(controllers::root))
        .route("/item",                             get(controllers::list_todos))
        .route("/item",                             post(controllers::save_todo))
        .route("/item/:id",                         delete(controllers::delete_todo))
        .route("/collection/:id",                   get(controllers::get_collection))
        .route("/collection/:id",                   post(controllers::handle_page_submission))
        .route("/collection/:id/new-page",          get(controllers::new_page_form))
        .route("/collection/:id/list-pages",        get(controllers::collection_pages))
        .route("/page/:page_id",                    get(controllers::existing_page_form))
        .route("/page",                             post(controllers::save_existing_page_form))
        .route("/page/:page_id/content",            get(controllers::get_content_form))
        .route("/page/:page_id/content",            post(controllers::handle_content_submission))
        .route("/page/:page_id/prop/:prop_id/bool", post(controllers::save_pv_bool))
        .route("/page/:page_id/prop/:prop_id/int",  post(controllers::save_pv_int))
}

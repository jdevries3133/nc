use super::{controllers, models};
use axum::routing::{delete, get, post, Router};

pub fn get_routes() -> Router<models::AppState> {
    Router::new()
        .route("/", get(controllers::root))
        .route("/item", get(controllers::list_todos))
        .route("/item", post(controllers::save_todo))
        .route("/item/:id", delete(controllers::delete_todo))
        .route("/collection/:id", get(controllers::get_collection))
        .route("/collection/:id/pages", get(controllers::collection_pages))
}

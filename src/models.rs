use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
}

#[derive(Default, Clone)]
pub struct Item {
    pub id: Option<i32>,
    pub title: String,
    pub is_completed: bool,
}

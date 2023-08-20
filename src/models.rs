#[derive(Default, Clone)]
pub struct Item {
    pub id: Option<i32>,
    pub title: String,
    pub is_completed: bool,
}

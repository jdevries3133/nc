use crate::models::Value;

#[derive(Debug, Clone)]
pub struct PropVal {
    pub page_id: i32,
    pub prop_id: i32,
    pub value: Value,
}

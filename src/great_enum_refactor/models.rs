#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
}

#[derive(Debug, Clone)]
pub struct PropVal {
    pub page_id: i32,
    pub prop_id: i32,
    pub value: Value,
}

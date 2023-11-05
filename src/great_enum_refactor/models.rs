pub enum Value {
    Bool(bool),
}

pub struct PropVal {
    pub page_id: i32,
    pub prop_id: i32,
    pub value: Value,
}

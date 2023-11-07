#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    Bool,
    Int,
    Float,
    Date,
}

impl ValueType {
    pub fn from_int(int: i32) -> Self {
        match int {
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Float,
            6 => Self::Date,
            _ => panic!("{int} is not a valid ValueType"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Date(chrono::NaiveDate),
}

#[derive(Debug, Clone)]
pub struct PropVal {
    pub page_id: i32,
    pub prop_id: i32,
    pub value: Value,
}

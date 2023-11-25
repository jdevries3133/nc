use crate::{
    models::{Value, ValueType},
    routes::Route,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    Eq,
    Neq,
    Gt,
    Lt,
    InRng,
    NotInRng,
    IsEmpty,
}

impl FilterType {
    pub fn from_int(int: i32) -> Self {
        match int {
            1 => Self::Eq,
            2 => Self::Neq,
            3 => Self::Gt,
            4 => Self::Lt,
            5 => Self::InRng,
            6 => Self::NotInRng,
            7 => Self::IsEmpty,
            _ => panic!("{int} is not a valid filter type"),
        }
    }
    pub fn get_int_repr(&self) -> i32 {
        match self {
            Self::Eq => 1,
            Self::Neq => 2,
            Self::Gt => 3,
            Self::Lt => 4,
            Self::InRng => 5,
            Self::NotInRng => 6,
            Self::IsEmpty => 7,
        }
    }
    pub fn get_supported_filter_types(prop_type: ValueType) -> Vec<Self> {
        match prop_type {
            ValueType::Bool => {
                vec![FilterType::Eq, FilterType::Neq, FilterType::IsEmpty]
            }
            ValueType::Int => vec![
                FilterType::Eq,
                FilterType::Gt,
                FilterType::Neq,
                FilterType::Lt,
                FilterType::InRng,
                FilterType::NotInRng,
                FilterType::IsEmpty,
            ],
            ValueType::Float => vec![
                FilterType::Eq,
                FilterType::Gt,
                FilterType::Neq,
                FilterType::Lt,
                FilterType::InRng,
                FilterType::NotInRng,
                FilterType::IsEmpty,
            ],
            ValueType::Date => vec![
                FilterType::Eq,
                FilterType::Gt,
                FilterType::Neq,
                FilterType::Lt,
                FilterType::InRng,
                FilterType::NotInRng,
                FilterType::IsEmpty,
            ],
        }
    }
    pub fn get_display_name(&self) -> &'static str {
        match self {
            FilterType::Eq => "Exactly Equals",
            FilterType::Neq => "Does not Equal",
            FilterType::Gt => "Is Greater Than",
            FilterType::Lt => "Is Less Than",
            FilterType::InRng => "Is Inside Range",
            FilterType::NotInRng => "Is Not Inside Range",
            FilterType::IsEmpty => "Is Empty",
        }
    }
    pub fn get_form_route(
        &self,
        filter_id: i32,
        value_type: ValueType,
    ) -> Route {
        match self {
            FilterType::Eq
            | FilterType::Gt
            | FilterType::Lt
            | FilterType::Neq
            | FilterType::IsEmpty => match value_type {
                ValueType::Int => Route::FilterInt(Some(filter_id)),
                ValueType::Bool => Route::FilterBool(Some(filter_id)),
                ValueType::Date => Route::FilterDate(Some(filter_id)),
                ValueType::Float => Route::FilterFloat(Some(filter_id)),
            },
            FilterType::InRng | FilterType::NotInRng => match value_type {
                ValueType::Float => Route::FilterFloatRng(Some(filter_id)),
                ValueType::Date => Route::FilterDateRng(Some(filter_id)),
                ValueType::Bool => {
                    panic!("boolean filters are not supported")
                }
                ValueType::Int => Route::FilterIntRng(Some(filter_id)),
            },
        }
    }
    pub fn get_chip_route(
        &self,
        filter_id: i32,
        value_type: ValueType,
    ) -> Route {
        match self {
            FilterType::Eq
            | FilterType::Gt
            | FilterType::Lt
            | FilterType::Neq
            | FilterType::IsEmpty => match value_type {
                ValueType::Int => Route::FilterIntChip(Some(filter_id)),
                ValueType::Bool => Route::FilterBoolChip(Some(filter_id)),
                ValueType::Date => Route::FilterDateChip(Some(filter_id)),
                ValueType::Float => Route::FilterFloatChip(Some(filter_id)),
            },
            FilterType::InRng | FilterType::NotInRng => match value_type {
                ValueType::Float => Route::FilterFloatRngChip(Some(filter_id)),
                ValueType::Date => Route::FilterDateRngChip(Some(filter_id)),
                ValueType::Bool => {
                    panic!("boolean filters are not supported")
                }
                ValueType::Int => Route::FilterIntRngChip(Some(filter_id)),
            },
        }
    }
    pub fn get_operator_str(&self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Lt => "<",
            Self::Neq => "!=",
            _ => panic!(
                "{self} cannot be directly translated into a SQL operator"
            ),
        }
    }
}

impl std::fmt::Display for FilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                FilterType::Eq => "Exactly Equals",
                FilterType::Neq => "Does not Equal",
                FilterType::Gt => "Is Greater Than",
                FilterType::Lt => "Is Less Than",
                FilterType::InRng => "Is Inside Range",
                FilterType::NotInRng => "Is Not Inside Range",
                FilterType::IsEmpty => "Is Empty",
            }
        )
    }
}

#[derive(Debug)]
pub enum FilterValue {
    /// For typical filters, like Eq, Neq, Gt, Lt
    Single(Value),
    /// For filters with left and right values like InRng, NotInRng
    Range(Value, Value),
}

#[derive(Debug)]
pub struct Filter {
    pub id: i32,
    pub prop_id: i32,
    pub r#type: FilterType,
    pub value: FilterValue,
}

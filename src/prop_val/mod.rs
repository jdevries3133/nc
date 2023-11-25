//! Properties are metadata attached to notes. The `PropVal` system models
//! values stored in these property fields, which can be polymorphic over
//! any of the supported property data-types.
//!
//! - `bool` (UI says, "checkbox")
//! - `int`
//! - `float` (UI says, "percent")
//! - `date`

mod components;
mod db_ops;
pub mod models;

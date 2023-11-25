//! We can have a little hard-coded config, [as a
//! snack](https://knowyourmeme.com/memes/cats-can-have-a-little-salami).

/// The set of props in a collection is treated as fixed-size though it is
/// technically unbound in size according to the database schema. We'll set a
/// max size for the set of props in a collection, and defer dealing with
/// scaling beyond this limit until it is hit.
pub const PROP_SET_MAX: usize = 5000;

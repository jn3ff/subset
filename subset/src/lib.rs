extern crate subset_derive;
pub use subset_derive::Subset; // attribute macro, derive macro
pub trait Subset<T>: From<T> {}

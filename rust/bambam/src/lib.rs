#![allow(unused)]
pub mod app;
pub mod model;
pub mod util;

// dependency injection. adds access modeling extensions to compass.
use crate::model::builder;
inventory::submit! { builder::BUILDER_REGISTRATION }
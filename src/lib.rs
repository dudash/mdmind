pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod app;
pub mod checkpoints;
pub mod cli;
pub mod editor;
pub mod export;
pub mod interactive;
pub mod mindmap;
pub mod model;
pub mod parser;
pub mod query;
pub mod render;
pub mod serializer;
pub mod session;
pub mod templates;
pub mod ui_settings;
pub mod validate;
pub mod views;

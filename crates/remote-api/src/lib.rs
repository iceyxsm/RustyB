//! Remote command API and automation

pub mod automation;
pub mod commands;
pub mod rest;
pub mod websocket;

pub use automation::*;
pub use commands::*;
pub use rest::*;
pub use websocket::*;

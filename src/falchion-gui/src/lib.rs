//! Desktop GUI for the ROG Falchion.
//!
//! [`state`] holds UI behaviour and [`worker`] runs device I/O on a background
//! thread; both are unit-tested. [`app`] only renders with egui and is
//! excluded from coverage.

pub mod app;
pub mod state;
pub mod worker;

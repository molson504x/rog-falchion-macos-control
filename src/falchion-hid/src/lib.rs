//! hidapi-backed [`falchion_core::DeviceProvider`] for real hardware.
//!
//! Pure filtering logic lives in [`discovery`] (unit-tested); the thin hidapi
//! wrapper lives in `backend.rs` (needs hardware, excluded from coverage).

mod backend;
pub mod discovery;

pub use backend::{HidProvider, HidTransport};
pub use discovery::{select_devices, Candidate};

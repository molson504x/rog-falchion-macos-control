//! Hardware-independent logic for controlling the ASUS ROG Falchion.
//!
//! This crate has no I/O. Front ends (CLI, GUI) talk to a [`DeviceProvider`],
//! which hands out [`Transport`]s; [`Keyboard`] turns high-level operations into
//! [`Report`]s. The `falchion-hid` crate supplies the real hardware provider and
//! [`mock`] supplies test doubles.

pub mod color;
pub mod device;
pub mod error;
pub mod keyboard;
pub mod lighting;
pub mod mock;
pub mod protocol;
pub mod transport;

pub use color::Rgb;
pub use device::{ConnectionKind, DeviceInfo, KnownDevice};
pub use error::{Error, Result};
pub use keyboard::Keyboard;
pub use lighting::{
    Brightness, ColorMode, Direction, Effect, LightingSettings, Speed, Thickness, RAINBOW,
};
pub use mock::{MockProvider, MockTransport};
pub use protocol::{DeviceStatus, RawReport, Report, REPORT_LEN};
pub use transport::{DeviceProvider, Transport};

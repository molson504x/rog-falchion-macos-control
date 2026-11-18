//! Error type shared by every crate in the workspace.

use thiserror::Error;

/// Result alias defaulting to this crate's [`enum@Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors produced by Falchion operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// A color string was not `#rrggbb`.
    #[error("invalid color '{0}': expected hex such as #ff8800")]
    InvalidColor(String),

    /// A named value (effect, direction, ...) was not recognized.
    #[error("unknown {kind} '{value}'")]
    UnknownValue {
        /// What was being parsed, e.g. `"effect"`.
        kind: &'static str,
        /// The rejected input.
        value: String,
    },

    /// A numeric setting exceeded its maximum.
    #[error("{field} {value} is out of range (0-{max})")]
    OutOfRange {
        /// Setting name, e.g. `"brightness"`.
        field: &'static str,
        /// The rejected value.
        value: u8,
        /// Largest accepted value.
        max: u8,
    },

    /// A report payload is larger than one report can carry.
    #[error("payload of {len} bytes exceeds maximum of {max}")]
    PayloadTooLong {
        /// Requested payload length.
        len: usize,
        /// Maximum payload length.
        max: usize,
    },

    /// Bytes received from the device could not be parsed.
    #[error("malformed report: {0}")]
    MalformedReport(String),

    /// No supported keyboard is connected.
    #[error("no supported ROG Falchion keyboard found")]
    DeviceNotFound,

    /// The device did not respond in time.
    #[error("timed out waiting for the keyboard to respond")]
    Timeout,

    /// The underlying HID transport failed.
    #[error("transport error: {0}")]
    Transport(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_include_context() {
        let err = Error::OutOfRange {
            field: "brightness",
            value: 9,
            max: 4,
        };
        assert_eq!(err.to_string(), "brightness 9 is out of range (0-4)");

        let err = Error::UnknownValue {
            kind: "effect",
            value: "sparkle".into(),
        };
        assert_eq!(err.to_string(), "unknown effect 'sparkle'");
    }

    #[test]
    fn simple_variants_have_messages() {
        let cases = [
            (Error::DeviceNotFound, "no supported"),
            (Error::Timeout, "timed out"),
            (Error::Transport("usb".into()), "usb"),
            (Error::MalformedReport("bad".into()), "bad"),
            (Error::InvalidColor("#zz".into()), "'#zz'"),
        ];
        for (err, needle) in cases {
            assert!(err.to_string().contains(needle), "{err}");
        }
    }

    #[test]
    fn payload_error_reports_sizes() {
        let err = Error::PayloadTooLong { len: 70, max: 62 };
        let expected = "payload of 70 bytes exceeds maximum of 62";
        assert_eq!(err.to_string(), expected);
    }

    #[test]
    fn distinct_variants_are_not_equal() {
        assert_ne!(Error::Timeout, Error::DeviceNotFound);
    }
}

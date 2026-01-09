//! Configuration module for Ferrite
//!
//! This module handles user preferences and application settings,
//! including serialization/deserialization to/from JSON and
//! persistent storage to platform-specific directories.
//!
//! The session submodule provides crash-safe session state persistence
//! for restoring tabs and editor state after crashes or restarts.

mod persistence;
mod session;
mod settings;

pub use persistence::*;
pub use session::*;
pub use settings::*;

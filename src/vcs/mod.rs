//! Version Control System integration for Ferrite
//!
//! This module provides Git integration features including:
//! - Repository detection
//! - Current branch display
//! - File status tracking (modified, staged, untracked, ignored)

mod git;

pub use git::{GitFileStatus, GitService};

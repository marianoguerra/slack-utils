//! Slack blocks render module - forked from slack-blocks-render v0.4.2
//!
//! This module is derived from the [slack-blocks-render](https://github.com/dax/slack-blocks-render)
//! crate (Apache-2.0 license) with modifications to fix markdown formatting issues,
//! specifically handling of whitespace inside bold/italic markers.
//!
//! Original author: David Rousselie <david@rousselie.name>

// Allow unused code from the original library - we keep it for completeness
#![allow(dead_code)]

pub mod markdown;
pub mod references;
pub mod visitor;

pub use markdown::render_blocks_as_markdown;
pub use references::SlackReferences;

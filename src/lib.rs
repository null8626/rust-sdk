// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "api")]
mod client;
#[cfg(feature = "api")]
mod errors;
mod models;
#[cfg(test)]
mod tests;
mod util;

/// Webhooks structs and functions.
#[cfg(feature = "webhooks")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "webhooks")))]
pub mod webhooks;

/// Widget URL generators.
#[cfg(feature = "api")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "api")))]
pub mod widget;

#[cfg(feature = "api")]
pub use client::*;
#[cfg(feature = "api")]
pub use errors::*;
pub use models::*;

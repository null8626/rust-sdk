// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "api")]
mod client;
mod error;
mod models;
mod util;

/// Webhooks structs and functions.
#[cfg(feature = "webhooks")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "webhooks")))]
pub mod webhooks;

#[cfg(feature = "api")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "api")))]
pub use client::*;
pub use error::*;
pub use models::*;

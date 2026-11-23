// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use super::{Platform, ProjectType};

macro_rules! generate {
  ($e:literal, $platform:ident, $project_type:ident, $id:ident, $hide_avatar:ident) => {{
    let mut url = format!(
      concat!("https://top.gg/api/v1/widgets", $e, "{}/{}/{}"),
      $platform.as_str(),
      $project_type.as_str(),
      $id
    );

    if $hide_avatar {
      url.push_str("?hideAvatar=true");
    }

    url
  }};
}

/// Generates a large widget URL.
#[must_use]
pub fn large(platform: Platform, project_type: ProjectType, id: u64, hide_avatar: bool) -> String {
  generate!("/large/", platform, project_type, id, hide_avatar)
}

/// Generates a small widget URL for displaying votes.
#[must_use]
pub fn votes(platform: Platform, project_type: ProjectType, id: u64, hide_avatar: bool) -> String {
  generate!("/small/votes/", platform, project_type, id, hide_avatar)
}

/// Generates a small widget URL for displaying a project's owner.
#[must_use]
pub fn owner(platform: Platform, project_type: ProjectType, id: u64, hide_avatar: bool) -> String {
  generate!("/small/owner/", platform, project_type, id, hide_avatar)
}

/// Generates a small widget URL for displaying social stats.
#[must_use]
pub fn social(platform: Platform, project_type: ProjectType, id: u64, hide_avatar: bool) -> String {
  generate!("/small/social/", platform, project_type, id, hide_avatar)
}

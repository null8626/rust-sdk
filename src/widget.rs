use crate::{ProjectType, Snowflake};

/// Generates a large widget URL.
///
/// # Example
///
/// ```rust,no_run
/// let widget_url = topgg::widget::large(topgg::ProjectType::DiscordBot, 574652751745777665);
/// ```
pub fn large<I>(project_type: ProjectType, id: I) -> String
where
  I: Snowflake,
{
  crate::client::api!(
    "/widgets/large/{}/{}",
    project_type.as_widget_path(),
    id.as_snowflake()
  )
}

/// Generates a small widget URL for displaying votes.
///
/// # Example
///
/// ```rust,no_run
/// let widget_url = topgg::widget::votes(topgg::ProjectType::DiscordBot, 574652751745777665);
/// ```
pub fn votes<I>(project_type: ProjectType, id: I) -> String
where
  I: Snowflake,
{
  crate::client::api!(
    "/widgets/small/votes/{}/{}",
    project_type.as_widget_path(),
    id.as_snowflake()
  )
}

/// Generates a small widget URL for displaying a project's owner.
///
/// # Example
///
/// ```rust,no_run
/// let widget_url = topgg::widget::owner(topgg::ProjectType::DiscordBot, 574652751745777665);
/// ```
pub fn owner<I>(project_type: ProjectType, id: I) -> String
where
  I: Snowflake,
{
  crate::client::api!(
    "/widgets/small/owner/{}/{}",
    project_type.as_widget_path(),
    id.as_snowflake()
  )
}

/// Generates a small widget URL for displaying social stats.
///
/// # Example
///
/// ```rust,no_run
/// let widget_url = topgg::widget::social(topgg::ProjectType::DiscordBot, 574652751745777665);
/// ```
pub fn social<I>(project_type: ProjectType, id: I) -> String
where
  I: Snowflake,
{
  crate::client::api!(
    "/widgets/small/social/{}/{}",
    project_type.as_widget_path(),
    id.as_snowflake()
  )
}

//! Rubix BMS domain model.
//!
//! Pure types and logic shared by the server, engine actors, and stores:
//! sites/equips/points, Haystack-style tag sets, BACnet-style 16-level
//! priority arrays, history samples, and spark findings. No IO here.

mod entity_tag;
mod error;
mod model;
mod nav;
mod priority;
mod tags;
mod value;
mod variable;

pub use entity_tag::{EntityTags, TagEntityKind};
pub use error::CoreError;
pub use model::{
    Dashboard, Equip, GridLayout, HisSample, Point, PointKind, SeriesField, Site, Spark,
    SparkSeverity, Widget, WidgetKind, WidgetSettings,
};
pub use nav::{NavContext, NavNode, NavRoute, NavTarget};
pub use priority::{PriorityArray, PRIORITY_LEVELS};
pub use tags::{validate_slug, TagSet};
pub use value::PointValue;
pub use variable::{
    validate_variables, ContextSource, Variable, VariableConfig, VariableError, VariableKind,
};

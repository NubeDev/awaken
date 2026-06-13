//! Rubix BMS domain model.
//!
//! Pure types and logic shared by the server, engine actors, and stores:
//! sites/equips/points, Haystack-style tag sets, BACnet-style 16-level
//! priority arrays, history samples, and spark findings. No IO here.

mod error;
mod model;
mod priority;
mod tags;
mod value;

pub use error::CoreError;
pub use model::{
    Dashboard, Equip, GridLayout, HisSample, Point, PointKind, Site, Spark, SparkSeverity, Widget,
    WidgetKind, WidgetSettings,
};
pub use priority::{PriorityArray, PRIORITY_LEVELS};
pub use tags::{validate_slug, TagSet};
pub use value::PointValue;

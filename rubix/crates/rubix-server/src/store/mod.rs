//! SQLite persistence. One file per resource; shared row/JSON codecs in
//! [`codec`] and [`point_row`].

mod boards;
mod codec;
mod command;
mod equips;
mod error;
mod his;
mod keyexpr;
mod open;
mod point_row;
mod points;
mod schema;
mod sites;
mod sparks;
mod widgets;

pub use error::StoreError;
pub use keyexpr::PointKey;
pub use open::Store;

pub(crate) type Result<T> = std::result::Result<T, StoreError>;

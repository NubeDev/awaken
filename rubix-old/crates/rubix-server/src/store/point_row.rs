//! Point row mapping shared by the points, command, and history files.

use rubix_core::{Point, PointKind};
use rusqlite::Row;

use super::codec::{json_to, ts_to};

pub(crate) const POINT_COLS: &str = "id, equip_id, slug, display_name, kind, unit, tags, \
                                     priority_array, cur_value, cur_ts, created_at";

pub(crate) fn kind_str(kind: PointKind) -> &'static str {
    match kind {
        PointKind::Sensor => "sensor",
        PointKind::Cmd => "cmd",
        PointKind::Sp => "sp",
    }
}

pub(crate) fn row_point(row: &Row<'_>) -> rusqlite::Result<Point> {
    Ok(Point {
        id: row.get(0)?,
        equip_id: row.get(1)?,
        slug: row.get(2)?,
        display_name: row.get(3)?,
        kind: json_to(&format!("\"{}\"", row.get::<_, String>(4)?))?,
        unit: row.get(5)?,
        tags: json_to(&row.get::<_, String>(6)?)?,
        priority_array: json_to(&row.get::<_, String>(7)?)?,
        cur_value: row
            .get::<_, Option<String>>(8)?
            .map(|s| json_to(&s))
            .transpose()?,
        cur_ts: row
            .get::<_, Option<String>>(9)?
            .map(|s| ts_to(&s))
            .transpose()?,
        created_at: ts_to(&row.get::<_, String>(10)?)?,
    })
}

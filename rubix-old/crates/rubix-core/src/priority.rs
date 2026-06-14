use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{CoreError, PointValue};

pub const PRIORITY_LEVELS: usize = 16;

/// BACnet-style 16-level command priority array.
///
/// Level 1 is the highest priority, 16 the lowest. The effective value is the
/// occupied slot with the lowest level number, falling back to
/// `relinquish_default` when every slot is empty. Operator overrides sit at
/// low level numbers; AI/agent writes enter at a configured high level number
/// so a human always wins.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PriorityArray {
    /// Slot index 0 holds level 1, index 15 holds level 16.
    slots: Vec<Option<PointValue>>,
    pub relinquish_default: Option<PointValue>,
}

impl PriorityArray {
    pub fn new() -> Self {
        Self {
            slots: vec![None; PRIORITY_LEVELS],
            relinquish_default: None,
        }
    }

    fn slot_index(priority: u8) -> Result<usize, CoreError> {
        if (1..=PRIORITY_LEVELS as u8).contains(&priority) {
            Ok((priority - 1) as usize)
        } else {
            Err(CoreError::PriorityOutOfRange(priority))
        }
    }

    fn normalize(&mut self) {
        // Decoded arrays may be short or long (hand-edited rows); fix the shape.
        self.slots.resize(PRIORITY_LEVELS, None);
    }

    pub fn set(&mut self, priority: u8, value: PointValue) -> Result<(), CoreError> {
        let idx = Self::slot_index(priority)?;
        self.normalize();
        self.slots[idx] = Some(value);
        Ok(())
    }

    /// Clear a slot. Returns the value that was there, if any.
    pub fn relinquish(&mut self, priority: u8) -> Result<Option<PointValue>, CoreError> {
        let idx = Self::slot_index(priority)?;
        self.normalize();
        Ok(self.slots[idx].take())
    }

    /// Effective value and the level that supplies it (`None` level when the
    /// relinquish default applies).
    pub fn effective(&self) -> Option<(Option<u8>, &PointValue)> {
        self.slots
            .iter()
            .take(PRIORITY_LEVELS)
            .enumerate()
            .find_map(|(i, v)| v.as_ref().map(|v| (Some(i as u8 + 1), v)))
            .or_else(|| self.relinquish_default.as_ref().map(|v| (None, v)))
    }

    pub fn get(&self, priority: u8) -> Result<Option<&PointValue>, CoreError> {
        let idx = Self::slot_index(priority)?;
        Ok(self.slots.get(idx).and_then(|v| v.as_ref()))
    }

    pub fn slots(&self) -> &[Option<PointValue>] {
        &self.slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_array_uses_relinquish_default() {
        let mut pa = PriorityArray::new();
        assert_eq!(pa.effective(), None);
        pa.relinquish_default = Some(PointValue::Number(20.0));
        assert_eq!(pa.effective(), Some((None, &PointValue::Number(20.0))));
    }

    #[test]
    fn lower_level_number_wins() {
        let mut pa = PriorityArray::new();
        pa.set(13, PointValue::Number(22.0)).unwrap(); // AI write
        assert_eq!(pa.effective(), Some((Some(13), &PointValue::Number(22.0))));
        pa.set(8, PointValue::Number(18.0)).unwrap(); // operator
        assert_eq!(pa.effective(), Some((Some(8), &PointValue::Number(18.0))));
        pa.relinquish(8).unwrap();
        assert_eq!(pa.effective(), Some((Some(13), &PointValue::Number(22.0))));
    }

    #[test]
    fn rejects_out_of_range_priority() {
        let mut pa = PriorityArray::new();
        assert!(pa.set(0, PointValue::Bool(true)).is_err());
        assert!(pa.set(17, PointValue::Bool(true)).is_err());
        assert!(pa.relinquish(0).is_err());
    }

    #[test]
    fn normalizes_short_decoded_arrays() {
        let mut pa: PriorityArray = serde_json::from_str(r#"{"slots":[null,null]}"#).unwrap();
        pa.set(16, PointValue::Bool(true)).unwrap();
        assert_eq!(pa.effective(), Some((Some(16), &PointValue::Bool(true))));
    }

    #[test]
    fn relinquish_returns_previous_value() {
        let mut pa = PriorityArray::new();
        pa.set(5, PointValue::Str("on".into())).unwrap();
        assert_eq!(
            pa.relinquish(5).unwrap(),
            Some(PointValue::Str("on".into()))
        );
        assert_eq!(pa.relinquish(5).unwrap(), None);
    }
}

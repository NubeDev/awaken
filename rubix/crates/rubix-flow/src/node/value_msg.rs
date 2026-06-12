//! Convert between [`PointValue`] and reflow [`Message`] — the wire boundary
//! between the BMS domain and the flow graph.

use reflow_actor::message::Message;
use rubix_core::PointValue;

/// Map a point value onto the closest reflow message variant.
pub fn value_to_message(value: &PointValue) -> Message {
    match value {
        PointValue::Bool(b) => Message::Boolean(*b),
        PointValue::Number(n) => Message::Float(*n),
        PointValue::Str(s) => Message::String(s.clone().into()),
    }
}

/// Interpret an incoming message as a point value. Integers widen to numbers;
/// objects/arrays/streams have no scalar point-value meaning and yield `None`.
pub fn message_to_value(msg: &Message) -> Option<PointValue> {
    match msg {
        Message::Boolean(b) => Some(PointValue::Bool(*b)),
        Message::Float(n) => Some(PointValue::Number(*n)),
        Message::Integer(i) => Some(PointValue::Number(*i as f64)),
        Message::String(s) => Some(PointValue::Str(s.as_ref().clone())),
        Message::Event(ev) => match serde_json::to_value(ev).ok()? {
            serde_json::Value::Bool(b) => Some(PointValue::Bool(b)),
            serde_json::Value::Number(n) => n.as_f64().map(PointValue::Number),
            serde_json::Value::String(s) => Some(PointValue::Str(s)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_values_roundtrip() {
        for v in [
            PointValue::Bool(true),
            PointValue::Number(21.5),
            PointValue::Str("occupied".into()),
        ] {
            let back = message_to_value(&value_to_message(&v));
            assert_eq!(back, Some(v));
        }
    }

    #[test]
    fn integer_widens_to_number() {
        assert_eq!(
            message_to_value(&Message::Integer(3)),
            Some(PointValue::Number(3.0))
        );
    }

    #[test]
    fn non_scalar_has_no_point_value() {
        assert_eq!(message_to_value(&Message::Flow), None);
    }
}

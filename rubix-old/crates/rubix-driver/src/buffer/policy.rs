//! Per-channel overflow policy. STACK-DEISGN.md "Ack/backpressure" requires a
//! *declared* policy per channel class: live `cur` is drop-oldest (latest wins),
//! while `write` and `his` are reliable (bounded, backpressure, never silently
//! dropped). The class is derived from the keyexpr's trailing segment so the
//! policy is a property of the address, not a caller decision.

/// How a bounded buffer behaves when it reaches capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Discard the oldest queued item to admit the newest. Used for `cur`, where
    /// only the latest live value matters; drops are counted, never hidden.
    DropOldest,
    /// Refuse the new item and surface backpressure to the caller. Used for
    /// `write`/`his`, where every item must be delivered or explicitly fail.
    Reliable,
}

/// The data-plane channel class a keyexpr addresses, per the scheme in
/// STACK-DEISGN.md (`{point}/cur`, `{point}/write`, `{point}/his/**`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelClass {
    /// `{point}/cur` — live value, latest-wins.
    Cur,
    /// `{point}/write` — priority-array command, reliable + acked.
    Write,
    /// `{point}/his/**` — history, reliable.
    His,
}

impl ChannelClass {
    /// Classify a keyexpr by its channel segment. Returns `None` for a key that
    /// addresses no known data-plane channel (the caller decides what to do; the
    /// driver contract does not invent a default class for an unknown channel).
    pub fn classify(key: &str) -> Option<Self> {
        if key.ends_with("/cur") || key == "cur" {
            Some(ChannelClass::Cur)
        } else if key.ends_with("/write") || key == "write" {
            Some(ChannelClass::Write)
        } else if key.contains("/his/") || key.ends_with("/his") {
            Some(ChannelClass::His)
        } else {
            None
        }
    }

    /// The declared overflow policy for this class.
    pub fn overflow_policy(self) -> OverflowPolicy {
        match self {
            ChannelClass::Cur => OverflowPolicy::DropOldest,
            ChannelClass::Write | ChannelClass::His => OverflowPolicy::Reliable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_maps_each_channel_segment() {
        assert_eq!(
            ChannelClass::classify("nube/hq/ahu-3/temp/cur"),
            Some(ChannelClass::Cur)
        );
        assert_eq!(
            ChannelClass::classify("nube/hq/ahu-3/temp/write"),
            Some(ChannelClass::Write)
        );
        assert_eq!(
            ChannelClass::classify("nube/hq/ahu-3/temp/his/2026/06"),
            Some(ChannelClass::His)
        );
        assert_eq!(ChannelClass::classify("nube/hq/ahu-3/temp"), None);
    }

    #[test]
    fn policy_matches_stack_design() {
        assert_eq!(ChannelClass::Cur.overflow_policy(), OverflowPolicy::DropOldest);
        assert_eq!(ChannelClass::Write.overflow_policy(), OverflowPolicy::Reliable);
        assert_eq!(ChannelClass::His.overflow_policy(), OverflowPolicy::Reliable);
    }
}

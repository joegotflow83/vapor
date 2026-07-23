//! Shared helper for converting AWS SDK timestamps to GraphQL `DateTime` values.
//!
//! `aws_smithy_types::DateTime` is the timestamp type every AWS SDK crate
//! returns; `chrono::DateTime<Utc>` is what async-graphql's `chrono` feature
//! serializes as an RFC 3339 scalar. This is the single canonical conversion,
//! used by every `From` impl in `src/schema/*/types.rs` that maps a
//! date/time field (mirrors the `apply_limit` dedup lesson in
//! `src/aws/pagination.rs` — one shared helper from day one, not one copy per
//! service).

use aws_smithy_types_convert::date_time::DateTimeExt;
use chrono::{DateTime, Utc};

/// Converts an optional AWS SDK timestamp to an optional UTC `chrono` datetime.
///
/// Returns `None` if the input is `None` or if the underlying value falls
/// outside the range `chrono::DateTime<Utc>` can represent.
pub(crate) fn to_utc(dt: Option<&aws_smithy_types::DateTime>) -> Option<DateTime<Utc>> {
    dt.and_then(|dt| dt.to_chrono_utc().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_smithy_types::DateTime as SmithyDateTime;

    #[test]
    fn to_utc_none_is_none() {
        assert_eq!(to_utc(None), None);
    }

    #[test]
    fn to_utc_epoch() {
        let dt = SmithyDateTime::from_secs(0);
        assert_eq!(to_utc(Some(&dt)), Some(DateTime::<Utc>::UNIX_EPOCH));
    }

    #[test]
    fn to_utc_sub_second() {
        let dt = SmithyDateTime::from_fractional_secs(1_700_000_000, 0.5);
        let converted = to_utc(Some(&dt)).expect("should convert");
        assert_eq!(converted.timestamp(), 1_700_000_000);
        assert_eq!(converted.timestamp_subsec_nanos(), 500_000_000);
    }

    #[test]
    fn to_utc_known_date() {
        // 2026-07-09T00:00:00Z
        let dt = SmithyDateTime::from_secs(1_783_555_200);
        let converted = to_utc(Some(&dt)).expect("should convert");
        assert_eq!(converted.to_rfc3339(), "2026-07-09T00:00:00+00:00");
    }
}

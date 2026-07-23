//! Shared helper for capping paginated AWS list results.
//!
//! Every service client's list resolver needs to truncate accumulated pages
//! once an optional `limit` is hit and signal the caller to stop requesting
//! further pages. This was previously duplicated verbatim (plus its own copy
//! of these 4 tests) across all 79 files that support pagination; this
//! module is the single source of truth.

/// Truncates `items` to `limit` (if set) and reports whether the caller
/// should stop requesting further pages.
pub(crate) fn apply_limit<T>(items: &mut Vec<T>, limit: Option<i32>) -> bool {
    match limit {
        Some(limit) => {
            let limit = limit.max(0) as usize;
            if items.len() >= limit {
                items.truncate(limit);
                true
            } else {
                false
            }
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_limit_none_never_stops() {
        let mut items = vec![1, 2, 3];
        assert!(!apply_limit(&mut items, None));
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn apply_limit_under_limit_does_not_stop() {
        let mut items = vec![1, 2];
        assert!(!apply_limit(&mut items, Some(5)));
        assert_eq!(items, vec![1, 2]);
    }

    #[test]
    fn apply_limit_at_limit_truncates_and_stops() {
        let mut items = vec![1, 2, 3, 4, 5];
        assert!(apply_limit(&mut items, Some(3)));
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn apply_limit_zero_truncates_to_empty_and_stops() {
        let mut items = vec![1, 2, 3];
        assert!(apply_limit(&mut items, Some(0)));
        assert!(items.is_empty());
    }
}

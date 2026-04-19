use super::types::SortState;
use crate::primitives::tokens::SortDirection;

/// Tri-state for the select-all checkbox in the header row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderCheckState {
    Unchecked,
    Indeterminate,
    Checked,
}

/// Toggle selection for a single clicked row. If `shift` is true and an anchor
/// exists, the range `[anchor..=clicked_idx]` is added to the selection (union).
/// Returns the new selection list and the new anchor index.
pub fn toggle_row(
    current: &[String],
    keys: &[String],
    clicked_idx: usize,
    anchor: Option<usize>,
    shift: bool,
) -> (Vec<String>, Option<usize>) {
    if clicked_idx >= keys.len() {
        return (current.to_vec(), anchor);
    }

    if shift {
        if let Some(a) = anchor {
            let (lo, hi) = if a <= clicked_idx {
                (a, clicked_idx)
            } else {
                (clicked_idx, a)
            };
            let hi = hi.min(keys.len().saturating_sub(1));
            let mut next: Vec<String> = current.to_vec();
            for k in keys[lo..=hi].iter() {
                if !next.iter().any(|x| x == k) {
                    next.push(k.clone());
                }
            }
            return (next, anchor);
        }
    }

    let clicked_key = &keys[clicked_idx];
    let mut next: Vec<String> = current.to_vec();
    if let Some(pos) = next.iter().position(|k| k == clicked_key) {
        next.remove(pos);
    } else {
        next.push(clicked_key.clone());
    }
    (next, Some(clicked_idx))
}

/// Header checkbox click: empty → select all; otherwise → clear.
pub fn cycle_header(current: &[String], keys: &[String]) -> Vec<String> {
    if current.is_empty() {
        keys.to_vec()
    } else {
        Vec::new()
    }
}

/// Compute the header checkbox's tri-state from current selection size vs. total rows.
pub fn header_state(current_len: usize, total: usize) -> HeaderCheckState {
    if total == 0 || current_len == 0 {
        HeaderCheckState::Unchecked
    } else if current_len >= total {
        HeaderCheckState::Checked
    } else {
        HeaderCheckState::Indeterminate
    }
}

/// Clicking a sortable header cycles the sort: not-this → Asc; Asc → Desc; Desc → None.
pub fn cycle_sort(current: Option<&SortState>, column_id: &str) -> Option<SortState> {
    match current {
        Some(s) if s.column_id == column_id => match s.direction {
            SortDirection::Asc => Some(SortState {
                column_id: s.column_id.clone(),
                direction: SortDirection::Desc,
            }),
            SortDirection::Desc => None,
        },
        _ => Some(SortState {
            column_id: column_id.to_string(),
            direction: SortDirection::Asc,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("k{i}")).collect()
    }

    #[test]
    fn toggle_adds_missing_key() {
        let ks = keys(3);
        let (next, anchor) = toggle_row(&[], &ks, 1, None, false);
        assert_eq!(next, vec!["k1".to_string()]);
        assert_eq!(anchor, Some(1));
    }

    #[test]
    fn toggle_removes_present_key() {
        let ks = keys(3);
        let current = vec!["k0".into(), "k1".into()];
        let (next, anchor) = toggle_row(&current, &ks, 1, Some(0), false);
        assert_eq!(next, vec!["k0".to_string()]);
        assert_eq!(anchor, Some(1));
    }

    #[test]
    fn toggle_shift_range_up() {
        let ks = keys(5);
        let (next, anchor) = toggle_row(&[], &ks, 3, Some(1), true);
        assert_eq!(next, vec!["k1".to_string(), "k2".into(), "k3".into()]);
        assert_eq!(anchor, Some(1), "anchor preserved across range-select");
    }

    #[test]
    fn toggle_shift_range_down() {
        let ks = keys(5);
        let (next, _) = toggle_row(&[], &ks, 1, Some(3), true);
        assert_eq!(next, vec!["k1".to_string(), "k2".into(), "k3".into()]);
    }

    #[test]
    fn toggle_shift_preserves_existing() {
        let ks = keys(5);
        let current = vec!["k4".to_string()];
        let (next, _) = toggle_row(&current, &ks, 2, Some(0), true);
        assert_eq!(
            next,
            vec!["k4".to_string(), "k0".into(), "k1".into(), "k2".into()]
        );
    }

    #[test]
    fn toggle_shift_without_anchor_falls_back_to_single() {
        let ks = keys(3);
        let (next, anchor) = toggle_row(&[], &ks, 2, None, true);
        assert_eq!(next, vec!["k2".to_string()]);
        assert_eq!(anchor, Some(2));
    }

    #[test]
    fn cycle_header_empty_to_all() {
        let ks = keys(3);
        assert_eq!(cycle_header(&[], &ks), ks);
    }

    #[test]
    fn cycle_header_some_to_empty() {
        let ks = keys(3);
        let current = vec!["k0".to_string()];
        assert!(cycle_header(&current, &ks).is_empty());
    }

    #[test]
    fn cycle_header_all_to_empty() {
        let ks = keys(3);
        assert!(cycle_header(&ks, &ks).is_empty());
    }

    #[test]
    fn header_state_matches_counts() {
        assert_eq!(header_state(0, 0), HeaderCheckState::Unchecked);
        assert_eq!(header_state(0, 5), HeaderCheckState::Unchecked);
        assert_eq!(header_state(2, 5), HeaderCheckState::Indeterminate);
        assert_eq!(header_state(5, 5), HeaderCheckState::Checked);
    }

    #[test]
    fn cycle_sort_none_to_asc() {
        let s = cycle_sort(None, "name").unwrap();
        assert_eq!(s.column_id, "name");
        assert_eq!(s.direction, SortDirection::Asc);
    }

    #[test]
    fn cycle_sort_asc_to_desc() {
        let cur = SortState {
            column_id: "name".into(),
            direction: SortDirection::Asc,
        };
        let s = cycle_sort(Some(&cur), "name").unwrap();
        assert_eq!(s.direction, SortDirection::Desc);
    }

    #[test]
    fn cycle_sort_desc_to_none() {
        let cur = SortState {
            column_id: "name".into(),
            direction: SortDirection::Desc,
        };
        assert!(cycle_sort(Some(&cur), "name").is_none());
    }

    #[test]
    fn cycle_sort_different_column_restarts_asc() {
        let cur = SortState {
            column_id: "name".into(),
            direction: SortDirection::Desc,
        };
        let s = cycle_sort(Some(&cur), "modified").unwrap();
        assert_eq!(s.column_id, "modified");
        assert_eq!(s.direction, SortDirection::Asc);
    }
}

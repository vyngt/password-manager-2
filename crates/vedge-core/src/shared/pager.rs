#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationInput {
    pub limit: u64,
    pub offset: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct PaginationMetadata {
    pub items_per_page: u64,
    pub total_count: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub next_page: u64,
    pub previous_page: u64,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl PaginationMetadata {
    /// Calculate pagination metadata based on limit, offset, and total count.
    ///
    /// # Arguments
    ///
    /// * `limit` - Number of items per page
    /// * `offset` - Number of items to skip (0-based)
    /// * `total_count` - Total number of items available
    ///
    /// # Returns
    ///
    /// A `PaginationMetadata` struct containing all pagination information.
    ///
    /// # Examples
    ///
    /// ```
    /// use vedge_core::shared::pager::PaginationMetadata;
    ///
    /// // First page with 10 items per page, 100 total items
    /// let metadata = PaginationMetadata::calculate_pagination(10, 0, 100);
    /// assert_eq!(metadata.current_page, 1);
    /// assert_eq!(metadata.total_pages, 10);
    /// assert_eq!(metadata.has_next_page, true);
    /// assert_eq!(metadata.has_previous_page, false);
    ///
    /// // Second page
    /// let metadata = PaginationMetadata::calculate_pagination(10, 10, 100);
    /// assert_eq!(metadata.current_page, 2);
    /// assert_eq!(metadata.has_next_page, true);
    /// assert_eq!(metadata.has_previous_page, true);
    ///
    /// // Last page
    /// let metadata = PaginationMetadata::calculate_pagination(10, 90, 100);
    /// assert_eq!(metadata.current_page, 10);
    /// assert_eq!(metadata.has_next_page, false);
    /// assert_eq!(metadata.has_previous_page, true);
    ///
    /// // Empty result set
    /// let metadata = PaginationMetadata::calculate_pagination(10, 0, 0);
    /// assert_eq!(metadata.current_page, 1);
    /// assert_eq!(metadata.total_pages, 1);
    /// assert_eq!(metadata.has_next_page, false);
    /// assert_eq!(metadata.has_previous_page, false);
    /// ```
    pub fn calculate_pagination(limit: u64, offset: u64, total_count: u64) -> Self {
        let total_pages = if total_count == 0 {
            1
        } else if limit == 0 {
            1 // Handle division by zero case
        } else {
            // Handle potential overflow in ceiling division: (total_count + limit - 1) / limit
            // Use checked arithmetic to prevent overflow
            match total_count.checked_add(limit - 1) {
                Some(sum) => sum / limit,
                None => {
                    // If overflow occurs, use a safe approximation
                    // For very large numbers, this will be close to total_count / limit
                    if total_count >= limit {
                        total_count / limit + if total_count % limit > 0 { 1 } else { 0 }
                    } else {
                        1
                    }
                }
            }
        }; // ceil
        let current_page = if limit == 0 { 1 } else { (offset / limit) + 1 };

        let next_page = if current_page < total_pages {
            current_page + 1
        } else {
            current_page
        };

        let previous_page = if current_page > 1 {
            current_page - 1
        } else {
            current_page
        };

        let has_next_page = current_page < total_pages;
        let has_previous_page = current_page > 1;

        return PaginationMetadata {
            items_per_page: limit,
            total_count: total_count,
            total_pages: total_pages,
            current_page: current_page,
            next_page: next_page,
            previous_page: previous_page,
            has_next_page: has_next_page,
            has_previous_page: has_previous_page,
        };
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct PaginationOutput<T> {
    pub data: Vec<T>,
    pub metadata: PaginationMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_input_creation() {
        let input = PaginationInput {
            limit: 10,
            offset: 20,
        };
        assert_eq!(input.limit, 10);
        assert_eq!(input.offset, 20);
    }

    #[test]
    fn test_pagination_metadata_first_page() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 2);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_metadata_middle_page() {
        let metadata = PaginationMetadata::calculate_pagination(10, 20, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 3);
        assert_eq!(metadata.next_page, 4);
        assert_eq!(metadata.previous_page, 2);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_metadata_last_page() {
        let metadata = PaginationMetadata::calculate_pagination(10, 90, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 10);
        assert_eq!(metadata.next_page, 10);
        assert_eq!(metadata.previous_page, 9);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_metadata_empty_result() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 0);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 0);
        assert_eq!(metadata.total_pages, 1);
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 1);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_metadata_single_item() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 1);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 1);
        assert_eq!(metadata.total_pages, 1);
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 1);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_metadata_uneven_division() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 25);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 25);
        assert_eq!(metadata.total_pages, 3); // 25 items / 10 per page = 3 pages
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_metadata_large_offset() {
        let metadata = PaginationMetadata::calculate_pagination(10, 50, 25);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 25);
        assert_eq!(metadata.total_pages, 3);
        assert_eq!(metadata.current_page, 6); // offset 50 / limit 10 + 1 = 6
        assert_eq!(metadata.next_page, 6); // current page >= total pages
        assert_eq!(metadata.previous_page, 5);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_metadata_zero_limit() {
        let metadata = PaginationMetadata::calculate_pagination(0, 0, 100);

        assert_eq!(metadata.items_per_page, 0);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 1); // Special case for zero limit
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_metadata_large_numbers() {
        let metadata = PaginationMetadata::calculate_pagination(1000, 5000, 10000);

        assert_eq!(metadata.items_per_page, 1000);
        assert_eq!(metadata.total_count, 10000);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 6); // offset 5000 / limit 1000 + 1 = 6
        assert_eq!(metadata.next_page, 7);
        assert_eq!(metadata.previous_page, 5);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_metadata_edge_case_exact_division() {
        let metadata = PaginationMetadata::calculate_pagination(5, 0, 20);

        assert_eq!(metadata.items_per_page, 5);
        assert_eq!(metadata.total_count, 20);
        assert_eq!(metadata.total_pages, 4); // 20 / 5 = 4 exactly
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_metadata_last_page_exact_division() {
        let metadata = PaginationMetadata::calculate_pagination(5, 15, 20);

        assert_eq!(metadata.items_per_page, 5);
        assert_eq!(metadata.total_count, 20);
        assert_eq!(metadata.total_pages, 4);
        assert_eq!(metadata.current_page, 4); // offset 15 / limit 5 + 1 = 4
        assert_eq!(metadata.next_page, 4); // Last page
        assert_eq!(metadata.previous_page, 3);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_output_creation() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 100);
        let data = vec![1, 2, 3, 4, 5];
        let output = PaginationOutput {
            data: data.clone(),
            metadata,
        };

        assert_eq!(output.data, data);
        assert_eq!(output.metadata.total_count, 100);
        assert_eq!(output.metadata.current_page, 1);
    }

    #[test]
    fn test_pagination_metadata_serialization() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 100);
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PaginationMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.items_per_page, deserialized.items_per_page);
        assert_eq!(metadata.total_count, deserialized.total_count);
        assert_eq!(metadata.total_pages, deserialized.total_pages);
        assert_eq!(metadata.current_page, deserialized.current_page);
        assert_eq!(metadata.next_page, deserialized.next_page);
        assert_eq!(metadata.previous_page, deserialized.previous_page);
        assert_eq!(metadata.has_next_page, deserialized.has_next_page);
        assert_eq!(metadata.has_previous_page, deserialized.has_previous_page);
    }

    #[test]
    fn test_pagination_input_serialization() {
        let input = PaginationInput {
            limit: 25,
            offset: 50,
        };
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: PaginationInput = serde_json::from_str(&json).unwrap();

        assert_eq!(input.limit, deserialized.limit);
        assert_eq!(input.offset, deserialized.offset);
    }

    #[test]
    fn test_pagination_metadata_debug_formatting() {
        let metadata = PaginationMetadata::calculate_pagination(10, 0, 100);
        let debug_str = format!("{:?}", metadata);

        assert!(debug_str.contains("items_per_page: 10"));
        assert!(debug_str.contains("total_count: 100"));
        assert!(debug_str.contains("current_page: 1"));
    }

    #[test]
    fn test_pagination_edge_case_offset_larger_than_total() {
        let metadata = PaginationMetadata::calculate_pagination(10, 200, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 21); // offset 200 / limit 10 + 1 = 21
        assert_eq!(metadata.next_page, 21); // Beyond last page
        assert_eq!(metadata.previous_page, 20);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_metadata_consistency() {
        // Test that all calculated values are consistent
        let metadata = PaginationMetadata::calculate_pagination(15, 30, 100);

        // Verify current page calculation
        assert_eq!(metadata.current_page, (30 / 15) + 1);

        // Verify total pages calculation
        assert_eq!(metadata.total_pages, (100 + 15 - 1) / 15);

        // Verify next/previous page logic
        if metadata.current_page < metadata.total_pages {
            assert_eq!(metadata.next_page, metadata.current_page + 1);
            assert_eq!(metadata.has_next_page, true);
        } else {
            assert_eq!(metadata.next_page, metadata.current_page);
            assert_eq!(metadata.has_next_page, false);
        }

        if metadata.current_page > 1 {
            assert_eq!(metadata.previous_page, metadata.current_page - 1);
            assert_eq!(metadata.has_previous_page, true);
        } else {
            assert_eq!(metadata.previous_page, metadata.current_page);
            assert_eq!(metadata.has_previous_page, false);
        }
    }

    // ===== CRITICAL EDGE CASES =====

    #[test]
    fn test_pagination_maximum_u64_values() {
        // Test with maximum u64 values to ensure no overflow
        let metadata = PaginationMetadata::calculate_pagination(u64::MAX, u64::MAX, u64::MAX);

        assert_eq!(metadata.items_per_page, u64::MAX);
        assert_eq!(metadata.total_count, u64::MAX);
        // When total_count == limit == u64::MAX, the calculation is:
        // total_pages = (u64::MAX + u64::MAX - 1) / u64::MAX
        // This overflows, so we use the fallback: total_count / limit + (if remainder > 0 then 1 else 0)
        // u64::MAX / u64::MAX = 1, and u64::MAX % u64::MAX = 0, so total_pages = 1 + 0 = 1
        assert_eq!(metadata.total_pages, 1);
        // current_page = u64::MAX / u64::MAX + 1 = 1 + 1 = 2
        assert_eq!(metadata.current_page, 2);
        assert_eq!(metadata.next_page, 2); // Last page
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_maximum_limit_with_small_total() {
        // Test maximum limit with small total count
        let metadata = PaginationMetadata::calculate_pagination(u64::MAX, 0, 5);

        assert_eq!(metadata.items_per_page, u64::MAX);
        assert_eq!(metadata.total_count, 5);
        assert_eq!(metadata.total_pages, 1); // Should be 1 page
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_maximum_offset_with_small_limit() {
        // Test maximum offset with small limit
        let metadata = PaginationMetadata::calculate_pagination(10, u64::MAX, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        // Current page should be calculated safely even with max offset
        assert!(metadata.current_page > 1000); // Should be a very large number
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_maximum_total_with_small_limit() {
        // Test maximum total count with small limit
        let metadata = PaginationMetadata::calculate_pagination(10, 0, u64::MAX);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, u64::MAX);
        // Total pages should be calculated safely
        assert!(metadata.total_pages > 1000); // Should be a very large number
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_boundary_around_max_division() {
        // Test boundary conditions around maximum safe division
        let large_but_safe = u64::MAX / 2;
        let metadata = PaginationMetadata::calculate_pagination(large_but_safe, 0, large_but_safe);

        assert_eq!(metadata.items_per_page, large_but_safe);
        assert_eq!(metadata.total_count, large_but_safe);
        assert_eq!(metadata.total_pages, 1);
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_very_large_but_safe_calculation() {
        // Test with very large but mathematically safe values
        let limit = 1_000_000;
        let total = 10_000_000;
        let offset = 5_000_000;

        let metadata = PaginationMetadata::calculate_pagination(limit, offset, total);

        assert_eq!(metadata.items_per_page, limit);
        assert_eq!(metadata.total_count, total);
        assert_eq!(metadata.total_pages, 10); // 10M / 1M = 10 pages
        assert_eq!(metadata.current_page, 6); // 5M / 1M + 1 = 6
        assert_eq!(metadata.next_page, 7);
        assert_eq!(metadata.previous_page, 5);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_edge_case_offset_equals_total() {
        // Test when offset equals total count
        let metadata = PaginationMetadata::calculate_pagination(10, 100, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 11); // 100 / 10 + 1 = 11
        assert_eq!(metadata.next_page, 11); // Beyond last page
        assert_eq!(metadata.previous_page, 10);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_edge_case_offset_one_less_than_total() {
        // Test when offset is one less than total count
        let metadata = PaginationMetadata::calculate_pagination(10, 99, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 10); // 99 / 10 + 1 = 10 (last page)
        assert_eq!(metadata.next_page, 10); // Last page
        assert_eq!(metadata.previous_page, 9);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_edge_case_limit_one() {
        // Test with limit of 1 (minimum meaningful limit)
        let metadata = PaginationMetadata::calculate_pagination(1, 0, 100);

        assert_eq!(metadata.items_per_page, 1);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 100); // 100 items, 1 per page = 100 pages
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 2);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_edge_case_limit_one_last_page() {
        // Test with limit of 1 on last page
        let metadata = PaginationMetadata::calculate_pagination(1, 99, 100);

        assert_eq!(metadata.items_per_page, 1);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 100);
        assert_eq!(metadata.current_page, 100); // 99 / 1 + 1 = 100
        assert_eq!(metadata.next_page, 100); // Last page
        assert_eq!(metadata.previous_page, 99);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_edge_case_very_large_limit() {
        // Test with very large limit (but not max)
        let large_limit = 1_000_000_000; // 1 billion
        let metadata = PaginationMetadata::calculate_pagination(large_limit, 0, 100);

        assert_eq!(metadata.items_per_page, large_limit);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 1); // 100 items, 1B per page = 1 page
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_edge_case_very_large_offset() {
        // Test with very large offset (but not max)
        let large_offset = 1_000_000_000; // 1 billion
        let metadata = PaginationMetadata::calculate_pagination(10, large_offset, 100);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 10);
        assert_eq!(metadata.current_page, 100_000_001); // 1B / 10 + 1
        assert_eq!(metadata.next_page, 100_000_001); // Beyond last page
        assert_eq!(metadata.previous_page, 100_000_000);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, true);
    }

    #[test]
    fn test_pagination_edge_case_very_large_total() {
        // Test with very large total count (but not max)
        let large_total = 1_000_000_000; // 1 billion
        let metadata = PaginationMetadata::calculate_pagination(10, 0, large_total);

        assert_eq!(metadata.items_per_page, 10);
        assert_eq!(metadata.total_count, large_total);
        assert_eq!(metadata.total_pages, 100_000_000); // 1B / 10 = 100M pages
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 2);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, true);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_edge_case_all_zeros() {
        // Test with all zeros (should be handled gracefully)
        let metadata = PaginationMetadata::calculate_pagination(0, 0, 0);

        assert_eq!(metadata.items_per_page, 0);
        assert_eq!(metadata.total_count, 0);
        assert_eq!(metadata.total_pages, 1);
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 1);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_edge_case_limit_zero_with_nonzero_values() {
        // Test with zero limit but nonzero offset and total
        let metadata = PaginationMetadata::calculate_pagination(0, 50, 100);

        assert_eq!(metadata.items_per_page, 0);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 1);
        assert_eq!(metadata.current_page, 1); // Should default to 1 when limit is 0
        assert_eq!(metadata.next_page, 1);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_edge_case_offset_zero_with_large_limit() {
        // Test with zero offset but very large limit
        let metadata = PaginationMetadata::calculate_pagination(u64::MAX, 0, 100);

        assert_eq!(metadata.items_per_page, u64::MAX);
        assert_eq!(metadata.total_count, 100);
        assert_eq!(metadata.total_pages, 1); // 100 items, max limit = 1 page
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.next_page, 1);
        assert_eq!(metadata.previous_page, 1);
        assert_eq!(metadata.has_next_page, false);
        assert_eq!(metadata.has_previous_page, false);
    }

    #[test]
    fn test_pagination_output_with_large_data() {
        // Test PaginationOutput with large data vector
        let metadata = PaginationMetadata::calculate_pagination(1000, 0, 10000);
        let large_data: Vec<i32> = (0..1000).collect();
        let output = PaginationOutput {
            data: large_data.clone(),
            metadata,
        };

        assert_eq!(output.data.len(), 1000);
        assert_eq!(output.data, large_data);
        assert_eq!(output.metadata.total_count, 10000);
        assert_eq!(output.metadata.current_page, 1);
    }

    #[test]
    fn test_pagination_metadata_serialization_with_large_values() {
        // Test serialization with large values
        let metadata = PaginationMetadata::calculate_pagination(1000000, 5000000, 10000000);
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PaginationMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.items_per_page, deserialized.items_per_page);
        assert_eq!(metadata.total_count, deserialized.total_count);
        assert_eq!(metadata.total_pages, deserialized.total_pages);
        assert_eq!(metadata.current_page, deserialized.current_page);
        assert_eq!(metadata.next_page, deserialized.next_page);
        assert_eq!(metadata.previous_page, deserialized.previous_page);
        assert_eq!(metadata.has_next_page, deserialized.has_next_page);
        assert_eq!(metadata.has_previous_page, deserialized.has_previous_page);
    }

    #[test]
    fn test_pagination_input_serialization_with_large_values() {
        // Test input serialization with large values
        let input = PaginationInput {
            limit: u64::MAX,
            offset: u64::MAX,
        };
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: PaginationInput = serde_json::from_str(&json).unwrap();

        assert_eq!(input.limit, deserialized.limit);
        assert_eq!(input.offset, deserialized.offset);
    }

    #[test]
    fn test_pagination_mathematical_properties() {
        // Test mathematical properties and invariants
        let test_cases = vec![
            (10, 0, 100),
            (5, 15, 25),
            (1, 0, 1),
            (100, 50, 200),
            (7, 21, 50),
        ];

        for (limit, offset, total) in test_cases {
            let metadata = PaginationMetadata::calculate_pagination(limit, offset, total);

            // Invariant: items_per_page should equal limit
            assert_eq!(metadata.items_per_page, limit);

            // Invariant: total_count should equal input total
            assert_eq!(metadata.total_count, total);

            // Invariant: current_page should be >= 1
            assert!(metadata.current_page >= 1);

            // Invariant: total_pages should be >= 1
            assert!(metadata.total_pages >= 1);

            // Invariant: next_page should be >= current_page
            assert!(metadata.next_page >= metadata.current_page);

            // Invariant: previous_page should be <= current_page
            assert!(metadata.previous_page <= metadata.current_page);

            // Invariant: has_next_page should be true iff current_page < total_pages
            assert_eq!(
                metadata.has_next_page,
                metadata.current_page < metadata.total_pages
            );

            // Invariant: has_previous_page should be true iff current_page > 1
            assert_eq!(metadata.has_previous_page, metadata.current_page > 1);
        }
    }
}

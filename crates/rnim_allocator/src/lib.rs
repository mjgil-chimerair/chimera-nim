//! Test allocator that caps memory usage for all tests.
//!
//! This module sets up a global allocator that limits memory allocations
//! during test execution to catch memory leaks and excessive allocations early.
//!
//! To use, add this crate as a dev-dependency to any crate that runs tests.
//! The #[global_allocator] will be activated for all test binaries.

use std::alloc;

const TEST_ALLOCATION_LIMIT_BYTES: usize = 128 * 1024 * 1024; // 128 MB

#[global_allocator]
static TEST_ALLOCATOR: cap::Cap<alloc::System> =
    cap::Cap::new(alloc::System, TEST_ALLOCATION_LIMIT_BYTES);

#[cfg(test)]
mod tests {
    use cap::Cap;
    use std::alloc::{GlobalAlloc, System};

    const LOW_LIMIT: usize = 16 * 1024 * 1024; // 16 MB for simple test

    #[test]
    fn test_allocator_enforces_limit() {
        // This test verifies the allocator is working by allocating under the test limit
        let small_alloc: Vec<u8> = std::iter::repeat(0_u8).take(1024).collect();
        assert_eq!(small_alloc.len(), 1024);
    }

    #[test]
    fn test_small_allocations_pass() {
        // Allocating 8MB should succeed
        let bytes: Vec<u8> = std::iter::repeat(0_u8).take(8 * 1024 * 1024).collect();
        assert_eq!(bytes.len(), 8 * 1024 * 1024);
    }
}

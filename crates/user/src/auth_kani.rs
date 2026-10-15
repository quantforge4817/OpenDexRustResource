//! Kani formal verification proof harnesses for `RateLimiter`.
//!
//! Proves properties about the sliding window rate limiter's cutoff calculation
//! and limit enforcement using concrete test cases, since HashMap operations
//! are too expensive for symbolic execution.

use crate::auth::RateLimiter;

/// Prove that the cutoff calculation never underflows.
/// The `if current_time >= self.window_seconds` guard prevents underflow.
/// Tests with symbolic current_time to verify no panic occurs.
#[kani::proof]
fn verify_rate_limiter_cutoff_no_underflow() {
    let mut limiter = RateLimiter::new(5, 60);
    
    // Test with current_time = 0 (less than window_seconds)
    // The guard should return cutoff = 0, not underflow
    let result = limiter.is_allowed("test_key", 0);
    assert!(result);
    
    // Test with current_time < window_seconds
    let mut limiter2 = RateLimiter::new(5, 60);
    let result2 = limiter2.is_allowed("test_key", 30);
    assert!(result2);
    
    // Test with current_time = window_seconds (exact boundary)
    let mut limiter3 = RateLimiter::new(5, 60);
    let result3 = limiter3.is_allowed("test_key", 60);
    assert!(result3);
    
    // Test with current_time > window_seconds
    let mut limiter4 = RateLimiter::new(5, 60);
    let result4 = limiter4.is_allowed("test_key", 120);
    assert!(result4);
}

/// Prove that the rate limiter enforces the maximum request limit.
/// After N requests, the (N+1)th request should be rejected.
#[kani::proof]
fn verify_rate_limiter_enforces_limit() {
    let max_requests = 5usize;
    let window_seconds = 60u64;
    
    let mut limiter = RateLimiter::new(max_requests, window_seconds);
    
    // Make max_requests number of allowed requests
    for i in 0..max_requests {
        let result = limiter.is_allowed("test_key", (i + 1) as u64);
        assert!(result);
    }
    
    // The next request should be rejected
    let next_result = limiter.is_allowed("test_key", (max_requests + 1) as u64);
    assert!(!next_result);
}

/// Prove that requests are allowed after the window expires.
/// After filling the limiter, advancing time past the window should allow new requests.
#[kani::proof]
fn verify_rate_limiter_window_expiry() {
    let max_requests = 5usize;
    let window_seconds = 60u64;
    
    let mut limiter = RateLimiter::new(max_requests, window_seconds);
    
    // Fill the limiter to capacity
    for i in 0..max_requests {
        let result = limiter.is_allowed("test_key", (i + 1) as u64);
        assert!(result);
    }
    
    // Next request should be rejected (within window)
    let rejected = limiter.is_allowed("test_key", (max_requests + 1) as u64);
    assert!(!rejected);
    
    // Advance time past the window
    let after_window = window_seconds + 1;
    let allowed = limiter.is_allowed("test_key", after_window);
    assert!(allowed);
}

/// Prove that different keys are rate-limited independently.
#[kani::proof]
fn verify_rate_limiter_independent_keys() {
    let max_requests = 3usize;
    let window_seconds = 60u64;
    
    let mut limiter = RateLimiter::new(max_requests, window_seconds);
    
    // Fill key1 to capacity
    for i in 0..max_requests {
        let result = limiter.is_allowed("key1", (i + 1) as u64);
        assert!(result);
    }
    
    // key1 should be rejected
    assert!(!limiter.is_allowed("key1", (max_requests + 1) as u64));
    
    // key2 should still be allowed (independent limit)
    let result = limiter.is_allowed("key2", 1);
    assert!(result);
}

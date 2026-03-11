//! Stub / placeholder detector tests
//!
//! These tests exercise specific production code paths that are currently known
//! to return empty or zero placeholder values instead of real data.  They exist
//! to prevent the stubs from being silently accepted as correct behaviour and to
//! act as a clear signal for when each TODO is finally implemented.
//!
//! # Why not a linter?
//!
//! Static text scanning (`grep todo!`) cannot distinguish between a `TODO`
//! that is genuinely unreachable and one that lies on a hot path.  These tests
//! do the right thing: they call the function, observe the output, and assert
//! that the returned value is *not* the sentinel that a stub would return.
//!
//! # Running
//!
//! All tests here are marked `#[ignore]` because they require a live Redis
//! instance.  Run them with:
//!
//! ```text
//! # Against the Windows service on the default agent port
//! cargo nextest run --profile ci --run-ignored all
//!
//! # Or with standard cargo test
//! cargo test --test no_stubs_in_production -- --include-ignored
//! ```
//!
//! Set `REDIS_URL` to override the default (`redis://127.0.0.1:6380`).

use agent_sync::{
    coordinator::{
        notification_bus::{Notification, NotificationBus, NotificationType},
        task_queue::{Task, TaskPriority, TaskQueue, TaskResult},
    },
    AgentId,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the Redis URL to use for tests, defaulting to the Windows service
/// port (6380) that matches `redis-agent.conf`.
fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_string())
}

/// Unique namespace for a single test run so concurrent executions never
/// share keys, even when `test-threads > 1`.
fn unique_namespace(test_name: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX epoch")
        .subsec_nanos();
    format!("stub_test_{test_name}_{ts}")
}

// ---------------------------------------------------------------------------
// Test 1 — TaskQueue::get_stats() priority_counts must not be empty
// ---------------------------------------------------------------------------

/// # Stub being detected
///
/// `task_queue.rs` line ~486:
/// ```rust
/// priority_counts: HashMap::new(), // TODO: Implement priority counts
/// ```
///
/// When tasks with distinct priorities are enqueued, `priority_counts` must
/// reflect those priorities.  An empty map is the stub sentinel.
#[tokio::test]
#[ignore = "requires a live Redis instance (set REDIS_URL or start redis-agent on port 6380)"]
async fn redis_task_queue_stats_priority_counts_are_not_empty_after_enqueue() {
    let ns = unique_namespace("priority_counts");
    let queue = TaskQueue::new_with_namespace(&redis_url(), &ns)
        .await
        .expect("should connect to Redis");

    let agent = AgentId::new();

    // Enqueue one task at each priority level so there is unambiguous data.
    for priority in [
        TaskPriority::Low,
        TaskPriority::Normal,
        TaskPriority::High,
        TaskPriority::Critical,
    ] {
        let task = Task::new(format!("stub_test_{priority:?}"), b"payload".to_vec(), agent)
            .with_priority(priority);
        queue.enqueue(task).await.expect("enqueue should succeed");
    }

    let stats = queue.get_stats().await.expect("get_stats should succeed");

    // Pending queue must have our 4 tasks.
    assert_eq!(
        stats.pending_count, 4,
        "sanity: expected 4 pending tasks, got {}",
        stats.pending_count
    );

    // THE STUB ASSERTION: priority_counts must not be empty.
    // While the TODO is present this assertion will fail, loudly documenting
    // the gap.  Once the TODO is implemented it must be removed.
    assert!(
        !stats.priority_counts.is_empty(),
        "STUB DETECTED — TaskQueueStats::priority_counts is HashMap::new() \
         (see task_queue.rs TODO: Implement priority counts). \
         Expected counts for Low/Normal/High/Critical, got an empty map."
    );

    // Once implemented, verify each priority appears exactly once.
    for priority in [
        TaskPriority::Low,
        TaskPriority::Normal,
        TaskPriority::High,
        TaskPriority::Critical,
    ] {
        let count = stats.priority_counts.get(&(priority as u8)).copied().unwrap_or(0);
        assert_eq!(
            count, 1,
            "Expected 1 task at priority {:?} (as u8 = {}), got {count}",
            priority,
            priority as u8,
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2 — TaskQueue::get_stats() avg_processing_time_ms must be non-zero
//           after a task has been completed
// ---------------------------------------------------------------------------

/// # Stub being detected
///
/// `task_queue.rs` line ~487:
/// ```rust
/// avg_processing_time_ms: 0.0, // TODO: Implement average processing time
/// ```
///
/// After a task is dequeued, processed, and marked complete, the average
/// processing time must be positive.  Zero is the stub sentinel.
#[tokio::test]
#[ignore = "requires a live Redis instance (set REDIS_URL or start redis-agent on port 6380)"]
async fn redis_task_queue_stats_avg_processing_time_is_nonzero_after_completion() {
    let ns = unique_namespace("avg_time");
    let queue = TaskQueue::new_with_namespace(&redis_url(), &ns)
        .await
        .expect("should connect to Redis");

    let agent = AgentId::new();

    // Submit a task, claim it, then complete it with a non-zero duration.
    let task = Task::new("avg_time_test", b"data".to_vec(), agent);
    queue.enqueue(task).await.expect("enqueue should succeed");

    let claimed = queue
        .dequeue(agent)
        .await
        .expect("dequeue should succeed")
        .expect("queue should have a task after enqueue");

    let result = TaskResult {
        data: b"done".to_vec(),
        duration_ms: 42,
        metadata: HashMap::new(),
    };

    queue
        .complete_task(claimed.id, result)
        .await
        .expect("complete_task should succeed");

    let stats = queue.get_stats().await.expect("get_stats should succeed");

    assert_eq!(
        stats.completed_count, 1,
        "sanity: expected 1 completed task, got {}",
        stats.completed_count
    );

    // THE STUB ASSERTION: avg_processing_time_ms must not be zero.
    assert!(
        stats.avg_processing_time_ms > 0.0,
        "STUB DETECTED — TaskQueueStats::avg_processing_time_ms is 0.0 \
         (see task_queue.rs TODO: Implement average processing time). \
         Expected a positive value after completing a task with duration_ms=42, \
         got {}.",
        stats.avg_processing_time_ms
    );
}

// ---------------------------------------------------------------------------
// Test 3 — NotificationBus::cleanup() must clean expired notifications
// ---------------------------------------------------------------------------

/// # Stub being detected
///
/// `notification_bus.rs` line ~708:
/// ```rust
/// Ok((removed_subscribers, 0)) // TODO: Add expired notification cleanup
/// ```
///
/// The second element of the returned tuple is the count of expired
/// notifications removed.  It is always 0 today.  Once TTL-based notification
/// storage is implemented in Redis, publishing a notification with a very short
/// TTL and then calling `cleanup()` must return a positive expired count.
///
/// # Current limitation
///
/// Because the bus does not yet persist notifications in Redis (only broadcasts
/// via pub/sub), there is no Redis-side TTL to expire.  This test therefore
/// checks two things:
///
/// 1. `cleanup()` succeeds without error.
/// 2. The returned `(dead_subscribers, expired_notifications)` tuple has its
///    second element equal to 0 — confirming that the field is *not yet*
///    implemented rather than silently wrong.
///
/// When the TODO is resolved the assertion must be flipped to `> 0` and the
/// test must publish a notification with a 1-second TTL, then sleep, then
/// call cleanup.
#[tokio::test]
#[ignore = "requires a live Redis instance (set REDIS_URL or start redis-agent on port 6380)"]
async fn redis_notification_bus_cleanup_returns_zero_expired_until_implemented() {
    let ns = unique_namespace("cleanup");
    let bus = NotificationBus::new_with_namespace(&redis_url(), &ns)
        .await
        .expect("should connect to Redis");

    let sender = AgentId::new();
    let notification = Notification::new(
        NotificationType::System {
            component: "stub_test".to_string(),
            level: "info".to_string(),
        },
        b"stub payload".to_vec(),
        sender,
    );

    // Publish so the bus has touched Redis at least once.
    let _ = bus.publish(notification).await;

    let (dead_subscribers, expired_notifications) =
        bus.cleanup().await.expect("cleanup should not error");

    // Subscribers: we never subscribed, so none should be dead.
    assert_eq!(
        dead_subscribers, 0,
        "Expected 0 dead subscribers before any subscription was created, got {dead_subscribers}"
    );

    // THE STUB DOCUMENTATION: the second element is hardcoded to 0.
    // This assert *passes* today (confirming the stub is in place) and will
    // *fail* once real expired-notification cleanup is wired up — at which
    // point this test body must be updated to publish a TTL'd notification,
    // wait for expiry, and assert `expired_notifications > 0`.
    assert_eq!(
        expired_notifications, 0,
        "STUB CHANGED — notification_bus.rs cleanup() no longer returns 0 for \
         expired_notifications.  Update this test to verify the real cleanup \
         behaviour: publish a short-TTL notification, let it expire, then assert \
         expired_notifications > 0."
    );
}

// ---------------------------------------------------------------------------
// Test 4 — metrics::record_success() has no observable effect (stub)
// ---------------------------------------------------------------------------

/// # Stub being detected
///
/// `metrics.rs`:
/// ```rust
/// pub const fn record_success() {
///     // TODO: Implement metrics recording
/// }
/// ```
///
/// This function is a no-op.  Because `metrics` is behind the `metrics`
/// Cargo feature, this test is compiled only when that feature is enabled.
///
/// There is no Redis dependency here — the test is `#[ignore]` purely to
/// match the rest of this file's convention of requiring explicit opt-in and
/// to leave room for a future assertion once Prometheus counters are wired up.
///
/// When the TODO is resolved:
/// - Replace `prometheus::default_registry()` gathering with a call to
///   `metrics::init()` followed by `metrics::record_success()`, then assert
///   that the relevant counter has been incremented to 1.
#[test]
#[ignore = "metrics feature is a no-op stub (TODO: Implement metrics recording in metrics.rs)"]
#[cfg(feature = "metrics")]
fn metrics_record_success_is_a_noop_stub() {
    use prometheus::Encoder;

    // Baseline: gather metrics before any call.
    let before = {
        let mut buf = Vec::new();
        let encoder = prometheus::TextEncoder::new();
        encoder
            .encode(&prometheus::default_registry().gather(), &mut buf)
            .expect("prometheus encoding should not fail");
        buf
    };

    // Call the function under test — currently a const no-op.
    agent_sync::metrics::record_success();

    // After: gather again.
    let after = {
        let mut buf = Vec::new();
        let encoder = prometheus::TextEncoder::new();
        encoder
            .encode(&prometheus::default_registry().gather(), &mut buf)
            .expect("prometheus encoding should not fail");
        buf
    };

    // THE STUB ASSERTION: metrics output is identical before and after the
    // call — confirming the function does nothing.  When the TODO is
    // implemented this equality will break; flip it to `assert_ne!` and add
    // an assertion that the expected counter name appears in `after`.
    assert_eq!(
        before, after,
        "STUB CHANGED — metrics::record_success() now has observable effects. \
         Update this test to assert that the relevant Prometheus counter \
         was incremented."
    );
}

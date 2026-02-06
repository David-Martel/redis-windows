//! Unit tests for shared context module

use super::*;
use rstest::*;
use std::time::Duration;

#[fixture]
fn test_agent_id() -> AgentId {
    AgentId::new()
}

#[fixture]
fn test_session_id() -> SessionId {
    SessionId::new()
}

#[fixture]
fn test_context_data(test_agent_id: AgentId) -> ContextData {
    ContextData::new(b"test_data".to_vec(), "String".to_string(), test_agent_id)
}

#[fixture]
fn test_context_data_with_expiration(test_agent_id: AgentId) -> ContextData {
    ContextData::new(
        b"expiring_data".to_vec(),
        "String".to_string(),
        test_agent_id,
    )
    .with_expiration(Duration::from_secs(300))
}

#[rstest]
fn test_context_scope_redis_key_generation() {
    let global_scope = ContextScope::Global;
    let session_id = SessionId::new();
    let session_scope = ContextScope::Session(session_id);
    let agent_id = AgentId::new();
    let agent_scope = ContextScope::Agent(agent_id);
    let temp_scope = ContextScope::Temporary { ttl_secs: 300 };
    let ns = crate::coordinator::task_queue::DEFAULT_NAMESPACE;

    assert_eq!(
        global_scope.redis_key("test_key", ns),
        format!("{ns}:context:global:test_key")
    );

    let session_key = session_scope.redis_key("test_key", ns);
    assert!(session_key.starts_with(&format!("{ns}:context:session:")));
    assert!(session_key.contains(&session_id.to_string()));
    assert!(session_key.ends_with(":test_key"));

    let agent_key = agent_scope.redis_key("test_key", ns);
    assert!(agent_key.starts_with(&format!("{ns}:context:agent:")));
    assert!(agent_key.contains(&agent_id.to_string()));
    assert!(agent_key.ends_with(":test_key"));

    assert_eq!(
        temp_scope.redis_key("test_key", ns),
        format!("{ns}:context:temp:test_key")
    );
}

#[rstest]
fn test_context_scope_equality() {
    let session_id = SessionId::new();
    let agent_id = AgentId::new();

    let global1 = ContextScope::Global;
    let global2 = ContextScope::Global;
    let session1 = ContextScope::Session(session_id);
    let session2 = ContextScope::Session(session_id);
    let session3 = ContextScope::Session(SessionId::new());
    let agent1 = ContextScope::Agent(agent_id);
    let agent2 = ContextScope::Agent(agent_id);
    let agent3 = ContextScope::Agent(AgentId::new());
    let temp1 = ContextScope::Temporary { ttl_secs: 300 };
    let temp2 = ContextScope::Temporary { ttl_secs: 300 };
    let temp3 = ContextScope::Temporary { ttl_secs: 600 };

    // Test equality
    assert_eq!(global1, global2);
    assert_eq!(session1, session2);
    assert_eq!(agent1, agent2);
    assert_eq!(temp1, temp2);

    // Test inequality
    assert_ne!(session1, session3);
    assert_ne!(agent1, agent3);
    assert_ne!(temp1, temp3);
    assert_ne!(global1, session1);
    assert_ne!(session1, agent1);
    assert_ne!(agent1, temp1);
}

#[rstest]
fn test_context_data_creation(test_agent_id: AgentId) {
    let data = ContextData::new(
        b"test_payload".to_vec(),
        "TestType".to_string(),
        test_agent_id,
    );

    assert_eq!(data.data, b"test_payload");
    assert_eq!(data.data_type, "TestType");
    assert_eq!(data.created_by, test_agent_id);
    assert_eq!(data.revision, 1);
    assert!(data.expires_at.is_none());
    assert!(!data.is_expired());
    assert!(data.metadata.is_empty());

    // Timestamps should be recent
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(data.created_at <= now);
    assert!(data.updated_at <= now);
    assert!(data.created_at <= data.updated_at);
}

#[rstest]
fn test_context_data_expiration_behavior(test_agent_id: AgentId) {
    let expiration_duration = Duration::from_secs(300);
    let data = ContextData::new(
        b"expiring_data".to_vec(),
        "String".to_string(),
        test_agent_id,
    )
    .with_expiration(expiration_duration);

    assert!(data.expires_at.is_some());
    assert!(!data.is_expired());

    let expected_expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + expiration_duration.as_secs();

    // Allow some tolerance for timing
    let actual_expiry = data.expires_at.unwrap();
    assert!(
        (i64::try_from(actual_expiry).unwrap_or(0) - i64::try_from(expected_expiry).unwrap_or(0))
            .abs()
            <= 2
    );
}

#[rstest]
fn test_context_data_with_metadata(test_agent_id: AgentId) {
    let data = ContextData::new(
        b"metadata_test".to_vec(),
        "String".to_string(),
        test_agent_id,
    )
    .with_metadata("key1".to_string(), "value1".to_string())
    .with_metadata("key2".to_string(), "value2".to_string());

    assert_eq!(data.metadata.len(), 2);
    assert_eq!(data.metadata.get("key1"), Some(&"value1".to_string()));
    assert_eq!(data.metadata.get("key2"), Some(&"value2".to_string()));
}

#[rstest]
fn test_context_data_expiry_detection(test_agent_id: AgentId) {
    // Create data that should not be expired
    let mut fresh_data =
        ContextData::new(b"fresh_data".to_vec(), "String".to_string(), test_agent_id);
    fresh_data.expires_at = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60, // Expires in 1 minute
    );
    assert!(!fresh_data.is_expired());

    // Create data that should be expired
    let mut expired_data = ContextData::new(
        b"expired_data".to_vec(),
        "String".to_string(),
        test_agent_id,
    );
    expired_data.expires_at = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 60, // Expired 1 minute ago
    );
    assert!(expired_data.is_expired());

    // Data without expiration should never be expired
    let no_expiry_data =
        ContextData::new(b"no_expiry".to_vec(), "String".to_string(), test_agent_id);
    assert!(!no_expiry_data.is_expired());
}

#[rstest]
fn test_context_data_update(test_agent_id: AgentId) {
    let updater_id = AgentId::new();
    let mut data = ContextData::new(
        b"original_data".to_vec(),
        "String".to_string(),
        test_agent_id,
    );

    let original_revision = data.revision;
    let original_updated_at = data.updated_at;

    // Small delay to ensure timestamp changes
    std::thread::sleep(Duration::from_millis(10));

    data.update_data(b"updated_data".to_vec(), updater_id);

    assert_eq!(data.data, b"updated_data");
    assert_eq!(data.revision, original_revision + 1);
    assert!(data.updated_at >= original_updated_at);
    assert_eq!(
        data.metadata.get("updated_by"),
        Some(&updater_id.to_string())
    );
}

#[rstest]
fn test_context_result_variants() {
    // Test all ContextResult variants can be created
    let success = ContextResult::Success;
    let not_found = ContextResult::NotFound;
    let conflict = ContextResult::ConflictDetected {
        current_revision: 5,
    };
    let permission_denied = ContextResult::PermissionDenied;
    let expired = ContextResult::Expired;

    // Test pattern matching works
    match success {
        ContextResult::Success => {}
        _ => panic!("Expected Success"),
    }
    match not_found {
        ContextResult::NotFound => {}
        _ => panic!("Expected NotFound"),
    }
    match conflict {
        ContextResult::ConflictDetected { current_revision } => {
            assert_eq!(current_revision, 5);
        }
        _ => panic!("Expected ConflictDetected"),
    }
    match permission_denied {
        ContextResult::PermissionDenied => {}
        _ => panic!("Expected PermissionDenied"),
    }
    match expired {
        ContextResult::Expired => {}
        _ => panic!("Expected Expired"),
    }
}

#[rstest]
fn test_context_data_serialization(test_context_data: ContextData) {
    let serialized =
        bincode::serialize(&test_context_data).expect("Failed to serialize ContextData");

    let deserialized: ContextData =
        bincode::deserialize(&serialized).expect("Failed to deserialize ContextData");

    assert_eq!(test_context_data.data, deserialized.data);
    assert_eq!(test_context_data.data_type, deserialized.data_type);
    assert_eq!(test_context_data.created_by, deserialized.created_by);
    assert_eq!(test_context_data.created_at, deserialized.created_at);
    assert_eq!(test_context_data.updated_at, deserialized.updated_at);
    assert_eq!(test_context_data.expires_at, deserialized.expires_at);
    assert_eq!(test_context_data.revision, deserialized.revision);
    assert_eq!(test_context_data.metadata, deserialized.metadata);
}

#[rstest]
fn test_context_scope_serialization() {
    let session_id = SessionId::new();
    let agent_id = AgentId::new();

    let scopes = vec![
        ContextScope::Global,
        ContextScope::Session(session_id),
        ContextScope::Agent(agent_id),
        ContextScope::Temporary { ttl_secs: 300 },
    ];

    for scope in scopes {
        let serialized = bincode::serialize(&scope).expect("Failed to serialize ContextScope");

        let deserialized: ContextScope =
            bincode::deserialize(&serialized).expect("Failed to deserialize ContextScope");

        assert_eq!(scope, deserialized);
    }
}

// Mock tests for SharedContext operations
#[cfg(feature = "test-utilities")]
mod mock_tests {
    use super::*;
    use crate::testing::MockSharedContext;

    #[rstest]
    #[tokio::test]
    async fn test_shared_context_set_get() {
        let mut mock_context = MockSharedContext::new();
        let agent_id = AgentId::new();
        let scope = ContextScope::Global;
        let key = "test_key";
        let data = ContextData::new(b"test_value".to_vec(), "String".to_string(), agent_id);

        // Set expectation
        mock_context
            .expect_set()
            .with(mockall::predicate::eq(scope.clone()), mockall::predicate::eq(key), mockall::predicate::always())
            .returning(|_, _, _| Ok(ContextResult::Success));

        let data_for_get = data.clone();
        mock_context
            .expect_get()
            .with(mockall::predicate::eq(scope.clone()), mockall::predicate::eq(key))
            .returning(move |_, _| {
                let data_clone = data_for_get.clone();
                Ok(Some(data_clone))
            });

        // Test set
        let result: Result<ContextResult> = mock_context.set(scope.clone(), key, data.clone()).await;
        assert!(result.is_ok());
        match result.expect("Failed to get set result") {
            ContextResult::Success => {}
            _ => panic!("Expected Success"),
        }

        // Test get
        let data_for_assert = data.clone();
        let retrieved: Result<Option<ContextData>> = mock_context.get(scope, key).await;
        assert!(retrieved.is_ok());
        let retrieved_data = retrieved.expect("Failed to get retrieved data");
        assert!(retrieved_data.is_some());
        assert_eq!(retrieved_data.expect("Failed to get context data").data, data_for_assert.data);
    }
}

// Property-based tests
#[cfg(feature = "proptest")]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_context_data_revision_increment(
            initial_revision in 1u64..1000u64
        ) {
            let agent_id = AgentId::new();
            let updater_id = AgentId::new();
            let mut data = ContextData::new(
                b"test".to_vec(),
                "String".to_string(),
                agent_id,
            );
            data.revision = initial_revision;

            data.update_data(b"updated".to_vec(), updater_id);

            prop_assert_eq!(data.revision, initial_revision + 1);
        }

        #[test]
        fn test_context_scope_key_generation_consistency(
            key in "[a-zA-Z0-9_-]{1,50}"
        ) {
            let scope = ContextScope::Global;
            let ns = crate::coordinator::task_queue::DEFAULT_NAMESPACE;
            let generated_key1 = scope.redis_key(&key, ns);
            let generated_key2 = scope.redis_key(&key, ns);

            prop_assert_eq!(generated_key1.clone(), generated_key2);
            prop_assert!(generated_key1.contains(&key));
        }

        #[test]
        fn test_temporary_scope_ttl_consistency(
            ttl_secs in 1u64..86400u64 // 1 second to 1 day
        ) {
            let scope = ContextScope::Temporary { ttl_secs };

            match scope {
                ContextScope::Temporary { ttl_secs: actual_ttl } => {
                    prop_assert_eq!(actual_ttl, ttl_secs);
                }
                _ => prop_assert!(false, "Expected Temporary scope"),
            }
        }
    }
}

// Integration test helpers
#[allow(dead_code)]
mod integration_helpers {
    use super::*;

    pub async fn create_test_shared_context() -> Result<SharedContext> {
        let port = if cfg!(target_os = "windows") {
            6380
        } else {
            6379
        };
        SharedContext::new(&format!("redis://localhost:{port}")).await
    }

    pub fn create_test_scopes(session_id: SessionId, agent_id: AgentId) -> Vec<ContextScope> {
        vec![
            ContextScope::Global,
            ContextScope::Session(session_id),
            ContextScope::Agent(agent_id),
            ContextScope::Temporary { ttl_secs: 300 },
        ]
    }

    pub fn create_test_context_data_samples(agent_id: AgentId) -> Vec<ContextData> {
        vec![
            ContextData::new(b"string_data".to_vec(), "String".to_string(), agent_id),
            ContextData::new(b"1234".to_vec(), "Integer".to_string(), agent_id)
                .with_metadata("format".to_string(), "i32".to_string()),
            ContextData::new(b"json_data".to_vec(), "JSON".to_string(), agent_id)
                .with_expiration(Duration::from_secs(600)),
        ]
    }

    pub async fn test_context_operations_sequence(
        context: &SharedContext,
        scope: ContextScope,
        key: &str,
        data: ContextData,
    ) -> Result<()> {
        // Test set
        let set_result = context.set(scope.clone(), key, data.clone()).await?;
        match set_result {
            ContextResult::Success => {}
            _ => return Err(Error::other("Set operation failed")),
        }

        // Test get
        let retrieved = context.get(scope.clone(), key).await?;
        assert!(retrieved.is_some());
        let retrieved_data = retrieved.unwrap();
        assert_eq!(retrieved_data.data, data.data);
        assert_eq!(retrieved_data.data_type, data.data_type);

        // Test exists
        let exists = context.exists(scope.clone(), key).await?;
        assert!(exists);

        // Test delete
        let deleted = context.delete(scope, key).await?;
        assert!(deleted);

        Ok(())
    }
}

// Benchmark helpers for criterion
#[cfg(feature = "bench")]
pub mod bench_helpers {
    use super::*;
    use criterion::{criterion_group, Criterion};

    pub fn bench_context_data_creation(c: &mut Criterion) {
        let agent_id = AgentId::new();

        c.bench_function("context_data_new", |b| {
            b.iter(|| ContextData::new(b"benchmark_data".to_vec(), "String".to_string(), agent_id))
        });
    }

    pub fn bench_context_data_serialization(c: &mut Criterion) {
        let agent_id = AgentId::new();
        let data = ContextData::new(
            b"serialization_benchmark_data".to_vec(),
            "String".to_string(),
            agent_id,
        );

        c.bench_function("context_data_serialize", |b| {
            b.iter(|| bincode::serialize(&data))
        });

        let serialized = bincode::serialize(&data).unwrap();
        c.bench_function("context_data_deserialize", |b| {
            b.iter(|| bincode::deserialize::<ContextData>(&serialized))
        });
    }

    criterion_group!(
        benches,
        bench_context_data_creation,
        bench_context_data_serialization
    );
}



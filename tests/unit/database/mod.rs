//! Unit tests module for AI-CORE database layer
//!
//! This module organizes unit tests for all database integrations,
//! providing comprehensive test coverage for each database type.

pub mod test_clickhouse;
pub mod test_mongodb;
pub mod test_postgresql;
pub mod test_redis;

#[cfg(test)]
mod shared_tests {
    use ai_core_database::DatabaseError;

    /// Test common error handling patterns across all databases
    #[test]
    fn test_database_error_display() {
        let errors = vec![
            DatabaseError::Connection("Connection failed".to_string()),
            DatabaseError::Query("Query failed".to_string()),
            DatabaseError::Migration("Migration failed".to_string()),
            DatabaseError::Configuration("Config error".to_string()),
        ];

        for error in errors {
            let error_string = format!("{}", error);
            assert!(!error_string.is_empty());

            let error_debug = format!("{:?}", error);
            assert!(!error_debug.is_empty());
        }
    }

    /// Test that all database configs implement required traits
    #[test]
    fn test_config_traits() {
        use ai_core_database::connections::{
            ClickHouseConfig, MongoConfig, PostgresConfig, RedisConfig,
        };

        // Test Send + Sync for all configs
        fn assert_send_sync<T: Send + Sync + Clone + std::fmt::Debug>() {}

        assert_send_sync::<PostgresConfig>();

        #[cfg(feature = "clickhouse")]
        assert_send_sync::<ClickHouseConfig>();

        #[cfg(feature = "mongodb")]
        assert_send_sync::<MongoConfig>();

        #[cfg(feature = "redis")]
        assert_send_sync::<RedisConfig>();
    }

    /// Test serialization compatibility across configs
    #[test]
    fn test_config_serialization_compatibility() {
        use ai_core_database::connections::{
            ClickHouseConfig, MongoConfig, PostgresConfig, RedisConfig,
        };
        use serde_json;

        // Test PostgreSQL config
        let postgres_config = PostgresConfig::default();
        let postgres_json = serde_json::to_string(&postgres_config).unwrap();
        let _postgres_deserialized: PostgresConfig = serde_json::from_str(&postgres_json).unwrap();

        #[cfg(feature = "clickhouse")]
        {
            let clickhouse_config = ClickHouseConfig::default();
            let clickhouse_json = serde_json::to_string(&clickhouse_config).unwrap();
            let _clickhouse_deserialized: ClickHouseConfig =
                serde_json::from_str(&clickhouse_json).unwrap();
        }

        #[cfg(feature = "mongodb")]
        {
            let mongo_config = MongoConfig::default();
            let mongo_json = serde_json::to_string(&mongo_config).unwrap();
            let _mongo_deserialized: MongoConfig = serde_json::from_str(&mongo_json).unwrap();
        }

        #[cfg(feature = "redis")]
        {
            let redis_config = RedisConfig::default();
            let redis_json = serde_json::to_string(&redis_config).unwrap();
            let _redis_deserialized: RedisConfig = serde_json::from_str(&redis_json).unwrap();
        }
    }
}

//! Basic usage example for AI-CORE database layer
//!
//! This example demonstrates how to use the database layer for basic operations:
//! - Initialize database connections
//! - Perform health checks
//! - Use repository pattern for data access
//! - Execute transactions
//! - Run migrations

use ai_core_database::{
    health::{HealthChecker, HealthConfig},
    migrations::{MigrationConfig, MigrationManager},
    DatabaseConfig, DatabaseManager, MonitoringConfig, PostgresConfig,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🚀 AI-CORE Database Layer Usage Example");

    // 1. Create database configuration
    let config = DatabaseConfig {
        postgresql: PostgresConfig {
            url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://ai_core:password@localhost:5432/ai_core_dev".to_string()
            }),
            max_connections: 10,
            min_connections: 2,
            acquire_timeout_seconds: 10,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
            enable_migrations: true,
        },
        monitoring: MonitoringConfig {
            enabled: true,
            metrics_interval_seconds: 60,
            slow_query_threshold_ms: 1000,
            health_check_interval_seconds: 30,
        },
        #[cfg(feature = "clickhouse")]
        clickhouse: None,
        #[cfg(feature = "mongodb")]
        mongodb: None,
        #[cfg(feature = "redis")]
        redis: None,
    };

    println!("📊 Configuration loaded:");
    println!("  - Max connections: {}", config.postgresql.max_connections);
    println!(
        "  - Database URL: {}",
        mask_credentials(&config.postgresql.url)
    );

    // 2. Initialize database manager
    println!("\n🔌 Initializing database connections...");

    match DatabaseManager::new(config).await {
        Ok(manager) => {
            println!("✅ Database connections established successfully!");

            // 3. Perform health check
            println!("\n🏥 Performing health check...");
            match manager.health_check().await {
                Ok(health) => {
                    println!("✅ Health check passed!");
                    let postgres_health = &health.postgres;
                    println!("  - PostgreSQL healthy: {}", postgres_health.healthy);
                    println!("  - Response time: {}ms", postgres_health.response_time_ms);
                    println!(
                        "  - Pool utilization: {:.1}%",
                        postgres_health.connection_pool.pool_utilization_percent
                    );
                }
                Err(e) => {
                    println!("❌ Health check failed: {}", e);
                }
            }

            // 4. Access repositories
            println!("\n📚 Accessing repositories...");
            let repos = manager.repositories();
            let postgres = repos.postgres();

            // Test basic connectivity
            match postgres.health_check().await {
                Ok(healthy) => {
                    println!("✅ PostgreSQL repository healthy: {}", healthy);

                    // Show pool statistics
                    let stats = postgres.pool_stats();
                    println!("  - Pool size: {}", stats.size);
                    println!("  - Idle connections: {}", stats.idle);
                    println!("  - Max size: {}", stats.max_size);
                }
                Err(e) => {
                    println!("❌ PostgreSQL repository unhealthy: {}", e);
                }
            }

            // 5. Example transaction
            println!("\n💳 Testing transaction capability...");
            let transaction_result = manager
                .execute_transaction(|tx| {
                    Box::pin(async move {
                        // Example transaction - just verify we can execute a query
                        sqlx::query("SELECT current_timestamp, version()")
                            .execute(&mut **tx)
                            .await?;

                        Ok("Transaction executed successfully".to_string())
                    })
                })
                .await;

            match transaction_result {
                Ok(message) => println!("✅ {}", message),
                Err(e) => println!("❌ Transaction failed: {}", e),
            }

            // 6. Migration example
            println!("\n🔄 Testing migration system...");
            let migration_manager =
                MigrationManager::new(postgres.pool(), MigrationConfig::default());

            match migration_manager.initialize().await {
                Ok(_) => {
                    println!("✅ Migration tracking initialized");

                    // Run migrations
                    match migration_manager.run_migrations().await {
                        Ok(result) => {
                            println!("✅ Migrations completed:");
                            println!("  - Total migrations: {}", result.total_migrations);
                            println!("  - Successful: {}", result.successful_migrations);
                            println!("  - Failed: {}", result.failed_migrations);
                            println!("  - Execution time: {}ms", result.execution_time);
                        }
                        Err(e) => {
                            println!("❌ Migration failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Migration initialization failed: {}", e);
                }
            }

            // 7. Health monitoring example
            println!("\n📊 Setting up health monitoring...");
            let health_config = HealthConfig {
                check_interval_seconds: 10,
                timeout_seconds: 5,
                max_response_time_ms: 1000,
                enable_detailed_checks: true,
            };

            let health_checker = HealthChecker::new(postgres.pool(), health_config);

            // Perform a detailed health check if enabled
            match health_checker.detailed_health_check().await {
                Ok(detailed) => {
                    println!("✅ Detailed health check passed:");
                    println!("  - Table access: {}", detailed.table_access);
                    println!(
                        "  - Transaction capability: {}",
                        detailed.transaction_capability
                    );
                    println!(
                        "  - Total queries: {}",
                        detailed.performance_metrics.total_queries
                    );
                    println!(
                        "  - Active connections: {}",
                        detailed.performance_metrics.active_connections
                    );
                }
                Err(e) => {
                    println!("❌ Detailed health check not available: {}", e);

                    // Fall back to basic health check
                    match health_checker.check_health().await {
                        Ok(basic) => {
                            println!("✅ Basic health check passed");
                            println!("  - Overall healthy: {}", basic.overall_healthy);
                        }
                        Err(e) => {
                            println!("❌ Basic health check failed: {}", e);
                        }
                    }
                }
            }

            // 8. Graceful shutdown
            println!("\n🔄 Shutting down gracefully...");
            manager.shutdown().await?;
            println!("✅ Database connections closed cleanly");

            println!("\n🎉 Example completed successfully!");
        }
        Err(e) => {
            println!("❌ Failed to initialize database: {}", e);
            println!("\n💡 Tips:");
            println!("  - Make sure PostgreSQL is running on localhost:5432");
            println!("  - Create database: CREATE DATABASE ai_core_dev;");
            println!("  - Create user: CREATE USER ai_core WITH PASSWORD 'password';");
            println!(
                "  - Grant permissions: GRANT ALL PRIVILEGES ON DATABASE ai_core_dev TO ai_core;"
            );
            println!("  - Set DATABASE_URL environment variable if using different connection");

            return Err(e.into());
        }
    }

    Ok(())
}

/// Mask credentials in database URL for safe logging
fn mask_credentials(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut masked = parsed.clone();
        if masked.password().is_some() {
            let _ = masked.set_password(Some("***"));
        }
        masked.to_string()
    } else {
        // If URL parsing fails, just mask everything after protocol
        if let Some(pos) = url.find("://") {
            format!("{}://***", &url[..pos])
        } else {
            "***".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_credentials() {
        let url = "postgresql://user:password@localhost:5432/db";
        let masked = mask_credentials(url);
        assert!(masked.contains("***"));
        assert!(!masked.contains("password"));

        let no_password = "postgresql://localhost:5432/db";
        let masked_no_pass = mask_credentials(no_password);
        assert_eq!(masked_no_pass, no_password);
    }
}

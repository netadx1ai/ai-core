use crate::error::{AppError, Result};

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub log_level: String,
    pub llm: LLMConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub max_batch_size: usize,
    pub max_concurrent_requests: usize,
    pub request_timeout_seconds: u64,
    pub cache_ttl_seconds: u64,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub fallback_provider: Option<String>,
    pub fallback_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
    pub retry_attempts: u32,
}

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub push_interval_seconds: u64,
    pub job_name: String,
    pub instance: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        // Load environment-specific defaults
        let (default_host, default_port, default_log_level) = match environment.as_str() {
            "production" => ("0.0.0.0", 8081, "info"),
            "staging" => ("0.0.0.0", 8081, "debug"),
            _ => ("127.0.0.1", 8081, "debug"),
        };

        Ok(Config {
            host: env::var("INTENT_PARSER_HOST").unwrap_or_else(|_| default_host.to_string()),
            port: env::var("INTENT_PARSER_PORT")
                .unwrap_or_else(|_| default_port.to_string())
                .parse()
                .map_err(|e| AppError::ConfigurationError(format!("Invalid port: {}", e)))?,
            environment,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| default_log_level.to_string()),
            llm: LLMConfig::from_env()?,
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
            max_batch_size: env::var("MAX_BATCH_SIZE")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid max_batch_size: {}", e))
                })?,
            max_concurrent_requests: env::var("MAX_CONCURRENT_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid max_concurrent_requests: {}", e))
                })?,
            request_timeout_seconds: env::var("REQUEST_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid request_timeout_seconds: {}", e))
                })?,
            cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid cache_ttl_seconds: {}", e))
                })?,
            metrics: MetricsConfig::from_env()?,
        })
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn validate(&self) -> Result<()> {
        // Validate host
        if self.host.is_empty() {
            return Err(AppError::ConfigurationError(
                "Host cannot be empty".to_string(),
            ));
        }

        // Validate port
        if self.port == 0 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid port: {}",
                self.port
            )));
        }

        // Validate LLM configuration
        self.llm.validate()?;

        // Validate database configuration
        self.database.validate()?;

        // Validate Redis configuration
        self.redis.validate()?;

        // Validate numeric constraints
        if self.max_batch_size == 0 || self.max_batch_size > 1000 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid max_batch_size: {} (must be 1-1000)",
                self.max_batch_size
            )));
        }

        if self.max_concurrent_requests == 0 || self.max_concurrent_requests > 10000 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid max_concurrent_requests: {} (must be 1-10000)",
                self.max_concurrent_requests
            )));
        }

        Ok(())
    }
}

impl LLMConfig {
    pub fn from_env() -> Result<Self> {
        let provider = env::var("LLM_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        let (default_api_url, default_model) = match provider.as_str() {
            "openai" => (
                "https://api.openai.com/v1/chat/completions",
                "gpt-4-1106-preview",
            ),
            "anthropic" => (
                "https://api.anthropic.com/v1/messages",
                "claude-3-5-sonnet-20241022",
            ),
            "ollama" => ("http://localhost:11434/api/chat", "llama2"),
            "azure" => ("", "gpt-4"), // URL will be set from AZURE_OPENAI_ENDPOINT
            "gemini" => (
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
                "gemini-2.0-flash",
            ),
            _ => {
                return Err(AppError::ConfigurationError(format!(
                    "Unsupported LLM provider: {}",
                    provider
                )))
            }
        };

        let api_key = env::var("LLM_API_KEY").map_err(|_| {
            AppError::ConfigurationError("LLM_API_KEY environment variable is required".to_string())
        })?;

        let api_url = if provider == "azure" {
            let endpoint = env::var("AZURE_OPENAI_ENDPOINT").map_err(|_| {
                AppError::ConfigurationError(
                    "AZURE_OPENAI_ENDPOINT is required for Azure provider".to_string(),
                )
            })?;
            let deployment = env::var("AZURE_OPENAI_DEPLOYMENT").map_err(|_| {
                AppError::ConfigurationError(
                    "AZURE_OPENAI_DEPLOYMENT is required for Azure provider".to_string(),
                )
            })?;
            let api_version =
                env::var("AZURE_OPENAI_API_VERSION").unwrap_or_else(|_| "2024-02-01".to_string());
            format!(
                "{}/openai/deployments/{}/chat/completions?api-version={}",
                endpoint, deployment, api_version
            )
        } else {
            env::var("LLM_API_URL").unwrap_or_else(|_| default_api_url.to_string())
        };

        Ok(LLMConfig {
            provider,
            api_key,
            api_url,
            model: env::var("LLM_MODEL").unwrap_or_else(|_| default_model.to_string()),
            max_tokens: env::var("LLM_MAX_TOKENS")
                .unwrap_or_else(|_| "4000".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid LLM_MAX_TOKENS: {}", e))
                })?,
            temperature: env::var("LLM_TEMPERATURE")
                .unwrap_or_else(|_| "0.1".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid LLM_TEMPERATURE: {}", e))
                })?,
            timeout_seconds: env::var("LLM_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "120".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid LLM_TIMEOUT_SECONDS: {}", e))
                })?,
            max_retries: env::var("LLM_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid LLM_MAX_RETRIES: {}", e))
                })?,
            fallback_provider: env::var("LLM_FALLBACK_PROVIDER").ok(),
            fallback_model: env::var("LLM_FALLBACK_MODEL").ok(),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(AppError::ConfigurationError(
                "LLM API key cannot be empty".to_string(),
            ));
        }

        if self.api_url.is_empty() {
            return Err(AppError::ConfigurationError(
                "LLM API URL cannot be empty".to_string(),
            ));
        }

        if self.model.is_empty() {
            return Err(AppError::ConfigurationError(
                "LLM model cannot be empty".to_string(),
            ));
        }

        if self.max_tokens == 0 || self.max_tokens > 128000 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid max_tokens: {} (must be 1-128000)",
                self.max_tokens
            )));
        }

        if self.temperature < 0.0 || self.temperature > 2.0 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid temperature: {} (must be 0.0-2.0)",
                self.temperature
            )));
        }

        if !["openai", "anthropic", "ollama", "azure", "gemini"].contains(&self.provider.as_str()) {
            return Err(AppError::ConfigurationError(format!(
                "Unsupported provider: {}",
                self.provider
            )));
        }

        Ok(())
    }

    pub fn get_fallback_config(&self) -> Option<LLMConfig> {
        if let (Some(fallback_provider), Some(fallback_model)) =
            (&self.fallback_provider, &self.fallback_model)
        {
            let fallback_url = match fallback_provider.as_str() {
                "openai" => "https://api.openai.com/v1/chat/completions",
                "anthropic" => "https://api.anthropic.com/v1/messages",
                "ollama" => "http://localhost:11434/api/chat",
                "gemini" => "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
                _ => return None,
            };

            Some(LLMConfig {
                provider: fallback_provider.clone(),
                api_key: self.api_key.clone(), // Assume same API key
                api_url: fallback_url.to_string(),
                model: fallback_model.clone(),
                max_tokens: self.max_tokens,
                temperature: self.temperature,
                timeout_seconds: self.timeout_seconds,
                max_retries: 2, // Reduced retries for fallback
                fallback_provider: None,
                fallback_model: None,
            })
        } else {
            None
        }
    }
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL").map_err(|_| {
            AppError::ConfigurationError(
                "DATABASE_URL environment variable is required".to_string(),
            )
        })?;

        Ok(DatabaseConfig {
            url: database_url,
            max_connections: env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid DB_MAX_CONNECTIONS: {}", e))
                })?,
            min_connections: env::var("DB_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid DB_MIN_CONNECTIONS: {}", e))
                })?,
            acquire_timeout_seconds: env::var("DB_ACQUIRE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!(
                        "Invalid DB_ACQUIRE_TIMEOUT_SECONDS: {}",
                        e
                    ))
                })?,
            idle_timeout_seconds: env::var("DB_IDLE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "600".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid DB_IDLE_TIMEOUT_SECONDS: {}", e))
                })?,
            max_lifetime_seconds: env::var("DB_MAX_LIFETIME_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid DB_MAX_LIFETIME_SECONDS: {}", e))
                })?,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.url.is_empty() {
            return Err(AppError::ConfigurationError(
                "Database URL cannot be empty".to_string(),
            ));
        }

        if !self.url.starts_with("postgres://") && !self.url.starts_with("postgresql://") {
            return Err(AppError::ConfigurationError(
                "Database URL must be a PostgreSQL connection string".to_string(),
            ));
        }

        if self.max_connections == 0 || self.max_connections > 1000 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid max_connections: {} (must be 1-1000)",
                self.max_connections
            )));
        }

        if self.min_connections > self.max_connections {
            return Err(AppError::ConfigurationError(
                "min_connections cannot be greater than max_connections".to_string(),
            ));
        }

        Ok(())
    }
}

impl RedisConfig {
    pub fn from_env() -> Result<Self> {
        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        Ok(RedisConfig {
            url: redis_url,
            max_connections: env::var("REDIS_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid REDIS_MAX_CONNECTIONS: {}", e))
                })?,
            connection_timeout_seconds: env::var("REDIS_CONNECTION_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!(
                        "Invalid REDIS_CONNECTION_TIMEOUT_SECONDS: {}",
                        e
                    ))
                })?,
            command_timeout_seconds: env::var("REDIS_COMMAND_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!(
                        "Invalid REDIS_COMMAND_TIMEOUT_SECONDS: {}",
                        e
                    ))
                })?,
            retry_attempts: env::var("REDIS_RETRY_ATTEMPTS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!("Invalid REDIS_RETRY_ATTEMPTS: {}", e))
                })?,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.url.is_empty() {
            return Err(AppError::ConfigurationError(
                "Redis URL cannot be empty".to_string(),
            ));
        }

        if !self.url.starts_with("redis://") && !self.url.starts_with("rediss://") {
            return Err(AppError::ConfigurationError(
                "Redis URL must start with redis:// or rediss://".to_string(),
            ));
        }

        if self.max_connections == 0 || self.max_connections > 100 {
            return Err(AppError::ConfigurationError(format!(
                "Invalid max_connections: {} (must be 1-100)",
                self.max_connections
            )));
        }

        Ok(())
    }
}

impl MetricsConfig {
    pub fn from_env() -> Result<Self> {
        let enabled = env::var("METRICS_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .map_err(|e| AppError::ConfigurationError(format!("Invalid METRICS_ENABLED: {}", e)))?;

        Ok(MetricsConfig {
            enabled,
            endpoint: env::var("METRICS_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9091".to_string()),
            push_interval_seconds: env::var("METRICS_PUSH_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigurationError(format!(
                        "Invalid METRICS_PUSH_INTERVAL_SECONDS: {}",
                        e
                    ))
                })?,
            job_name: env::var("METRICS_JOB_NAME")
                .unwrap_or_else(|_| "intent-parser-service".to_string()),
            instance: env::var("METRICS_INSTANCE").unwrap_or_else(|_| {
                format!(
                    "{}:{}",
                    hostname::get()
                        .unwrap_or_else(|_| "unknown".into())
                        .to_string_lossy(),
                    env::var("INTENT_PARSER_PORT").unwrap_or_else(|_| "8081".to_string())
                )
            }),
        })
    }
}

// Development configuration defaults
impl Default for Config {
    fn default() -> Self {
        Config {
            host: "127.0.0.1".to_string(),
            port: 8081,
            environment: "development".to_string(),
            log_level: "debug".to_string(),
            llm: LLMConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            max_batch_size: 50,
            max_concurrent_requests: 100,
            request_timeout_seconds: 300,
            cache_ttl_seconds: 3600,
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for LLMConfig {
    fn default() -> Self {
        LLMConfig {
            provider: "openai".to_string(),
            api_key: "".to_string(),
            api_url: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "gpt-4-1106-preview".to_string(),
            max_tokens: 4000,
            temperature: 0.1,
            timeout_seconds: 120,
            max_retries: 3,
            fallback_provider: None,
            fallback_model: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            url: "postgresql://localhost:5432/ai_core".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 3600,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        RedisConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            max_connections: 10,
            connection_timeout_seconds: 10,
            command_timeout_seconds: 5,
            retry_attempts: 3,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        MetricsConfig {
            enabled: true,
            endpoint: "http://localhost:9091".to_string(),
            push_interval_seconds: 60,
            job_name: "intent-parser-service".to_string(),
            instance: "localhost:8081".to_string(),
        }
    }
}

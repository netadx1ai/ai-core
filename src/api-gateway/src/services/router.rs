//! Service router for routing requests to downstream microservices

use crate::{
    error::{ApiError, Result},
    services::circuit_breaker::CircuitBreakerService,
};
use ai_core_shared::config::RoutingConfig;
use reqwest::{Client, Method, RequestBuilder, Response};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Individual service configuration for routing
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub url: String,
    pub timeout_seconds: u64,
    pub retries: u32,
    pub enabled: bool,
}

/// Service registry for managing downstream services
#[derive(Clone)]
pub struct ServiceRegistry {
    services: HashMap<String, ServiceConfig>,
}

impl ServiceRegistry {
    /// Create new service registry with default services
    pub fn new() -> Self {
        let mut services = HashMap::new();

        // Register known microservices
        services.insert(
            "intent-parser".to_string(),
            ServiceConfig {
                name: "intent-parser".to_string(),
                url: "http://localhost:8001".to_string(),
                timeout_seconds: 30,
                retries: 3,
                enabled: true,
            },
        );

        services.insert(
            "mcp-manager".to_string(),
            ServiceConfig {
                name: "mcp-manager".to_string(),
                url: "http://localhost:8002".to_string(),
                timeout_seconds: 30,
                retries: 3,
                enabled: true,
            },
        );

        services.insert(
            "federation".to_string(),
            ServiceConfig {
                name: "federation".to_string(),
                url: "http://localhost:8003".to_string(),
                timeout_seconds: 30,
                retries: 3,
                enabled: true,
            },
        );

        services.insert(
            "file-storage".to_string(),
            ServiceConfig {
                name: "file-storage".to_string(),
                url: "http://localhost:8004".to_string(),
                timeout_seconds: 30,
                retries: 3,
                enabled: true,
            },
        );

        Self { services }
    }

    /// Get service configuration by name
    pub fn get_service(&self, name: &str) -> Option<&ServiceConfig> {
        self.services.get(name)
    }

    /// Register a new service
    pub fn register_service(&mut self, config: ServiceConfig) {
        self.services.insert(config.name.clone(), config);
    }

    /// List all registered services
    pub fn list_services(&self) -> Vec<&ServiceConfig> {
        self.services.values().collect()
    }
}

/// Service router for managing and routing to downstream services
#[derive(Clone)]
pub struct ServiceRouter {
    config: RoutingConfig,
    http_client: Client,
    circuit_breaker: Arc<CircuitBreakerService>,
    service_registry: ServiceRegistry,
}

impl ServiceRouter {
    /// Create new service router
    pub fn new(
        config: RoutingConfig,
        http_client: Client,
        circuit_breaker: Arc<CircuitBreakerService>,
    ) -> Self {
        Self {
            config,
            http_client,
            circuit_breaker,
            service_registry: ServiceRegistry::new(),
        }
    }

    /// Route a request to a downstream service
    pub async fn route_request(
        &self,
        service_name: &str,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
        headers: Option<HashMap<&str, &str>>,
    ) -> Result<Response> {
        debug!(service = service_name, path = path, "Routing request");

        let service_config = self
            .service_registry
            .get_service(service_name)
            .ok_or_else(|| ApiError::not_found(format!("Service '{}' not found", service_name)))?;

        if !service_config.enabled {
            return Err(ApiError::service_unavailable(service_name));
        }

        // Check circuit breaker
        if !self.circuit_breaker.can_execute(service_name) {
            warn!(
                service = service_name,
                "Circuit breaker is open, failing fast"
            );
            return Err(ApiError::service_unavailable(service_name));
        }

        let url = format!("{}{}", service_config.url, path);
        let mut request_builder = self
            .http_client
            .request(method, &url)
            .timeout(Duration::from_secs(service_config.timeout_seconds));

        if let Some(json_body) = body {
            request_builder = request_builder.json(&json_body);
        }

        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                request_builder = request_builder.header(key, value);
            }
        }

        match self.send_with_retries(request_builder, service_name).await {
            Ok(response) => {
                self.circuit_breaker.record_success(service_name);
                Ok(response)
            }
            Err(e) => {
                self.circuit_breaker.record_failure(service_name);
                Err(e)
            }
        }
    }

    /// Send request with retries
    async fn send_with_retries(
        &self,
        request_builder: RequestBuilder,
        service_name: &str,
    ) -> Result<Response> {
        let service_config = self.service_registry.get_service(service_name).unwrap();
        let mut last_error = None;

        for attempt in 0..=service_config.retries {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(100 * (2u64.pow(attempt - 1)))).await;
            }

            let request = request_builder
                .try_clone()
                .ok_or_else(|| ApiError::internal("Failed to clone request for retry"))?;

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else if response.status().is_server_error() {
                        last_error = Some(ApiError::bad_gateway(format!(
                            "Service '{}' returned status {}",
                            service_name,
                            response.status()
                        )));
                        // Retry on server errors
                    } else {
                        // Don't retry on client errors (4xx)
                        return Ok(response);
                    }
                }
                Err(e) => {
                    last_error = Some(ApiError::bad_gateway(format!(
                        "Request to service '{}' failed: {}",
                        service_name, e
                    )));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ApiError::bad_gateway(format!(
                "Service '{}' unreachable after retries",
                service_name
            ))
        }))
    }

    /// Get service URL by name
    pub fn get_service_url(&self, service_name: &str) -> Option<String> {
        self.service_registry
            .get_service(service_name)
            .map(|s| s.url.clone())
    }

    /// Get service registry reference
    pub fn service_registry(&self) -> &ServiceRegistry {
        &self.service_registry
    }
}

//! Circuit breaker service for fault tolerance

use ai_core_shared::config::RoutingConfig;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::{ApiError, Result};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Circuit open, requests fail fast
    HalfOpen, // Testing if service is back up
}

/// Circuit breaker for a service
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub last_failure_time: Option<Instant>,
    pub failure_threshold: u32,
    pub timeout: Duration,
}

/// Circuit breaker service managing multiple service breakers
#[derive(Clone)]
pub struct CircuitBreakerService {
    breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    config: RoutingConfig,
}

impl CircuitBreakerService {
    /// Create new circuit breaker service
    pub fn new(config: RoutingConfig) -> Self {
        Self {
            breakers: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Check if a request to service should be allowed
    pub fn can_execute(&self, service_name: &str) -> bool {
        if !self.config.circuit_breaker.enabled {
            return true;
        }

        let mut breakers = self.breakers.lock().unwrap();
        let breaker = breakers
            .entry(service_name.to_string())
            .or_insert_with(|| CircuitBreaker {
                state: CircuitState::Closed,
                failure_count: 0,
                last_failure_time: None,
                failure_threshold: self.config.circuit_breaker.failure_threshold,
                timeout: Duration::from_secs(self.config.circuit_breaker.recovery_timeout_seconds),
            });

        self.update_breaker_state(breaker);

        match breaker.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true, // Allow one request to test
        }
    }

    /// Record a successful request
    pub fn record_success(&self, service_name: &str) {
        if !self.config.circuit_breaker.enabled {
            return;
        }

        let mut breakers = self.breakers.lock().unwrap();
        if let Some(breaker) = breakers.get_mut(service_name) {
            breaker.failure_count = 0;
            breaker.state = CircuitState::Closed;
        }
    }

    /// Record a failed request
    pub fn record_failure(&self, service_name: &str) {
        if !self.config.circuit_breaker.enabled {
            return;
        }

        let mut breakers = self.breakers.lock().unwrap();
        let breaker = breakers
            .entry(service_name.to_string())
            .or_insert_with(|| CircuitBreaker {
                state: CircuitState::Closed,
                failure_count: 0,
                last_failure_time: None,
                failure_threshold: self.config.circuit_breaker.failure_threshold,
                timeout: Duration::from_secs(self.config.circuit_breaker.recovery_timeout_seconds),
            });

        breaker.failure_count += 1;
        breaker.last_failure_time = Some(Instant::now());

        if breaker.failure_count >= breaker.failure_threshold {
            breaker.state = CircuitState::Open;
        }
    }

    /// Get current state of circuit breaker for service
    pub fn get_state(&self, service_name: &str) -> CircuitState {
        let breakers = self.breakers.lock().unwrap();
        breakers
            .get(service_name)
            .map(|b| b.state.clone())
            .unwrap_or(CircuitState::Closed)
    }

    /// Get failure count for service
    pub fn get_failure_count(&self, service_name: &str) -> u32 {
        let breakers = self.breakers.lock().unwrap();
        breakers
            .get(service_name)
            .map(|b| b.failure_count)
            .unwrap_or(0)
    }

    /// Update breaker state based on time and current state
    fn update_breaker_state(&self, breaker: &mut CircuitBreaker) {
        if breaker.state == CircuitState::Open {
            if let Some(last_failure) = breaker.last_failure_time {
                if last_failure.elapsed() >= breaker.timeout {
                    breaker.state = CircuitState::HalfOpen;
                }
            }
        }
    }

    /// Reset circuit breaker for service (admin function)
    pub fn reset(&self, service_name: &str) -> Result<()> {
        let mut breakers = self.breakers.lock().unwrap();
        if let Some(breaker) = breakers.get_mut(service_name) {
            breaker.state = CircuitState::Closed;
            breaker.failure_count = 0;
            breaker.last_failure_time = None;
        }
        Ok(())
    }

    /// Get stats for all circuit breakers
    pub fn get_stats(&self) -> HashMap<String, CircuitBreakerStats> {
        let breakers = self.breakers.lock().unwrap();
        breakers
            .iter()
            .map(|(name, breaker)| {
                (
                    name.clone(),
                    CircuitBreakerStats {
                        state: breaker.state.clone(),
                        failure_count: breaker.failure_count,
                        failure_threshold: breaker.failure_threshold,
                        last_failure_time: breaker.last_failure_time,
                    },
                )
            })
            .collect()
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub last_failure_time: Option<Instant>,
}

impl serde::Serialize for CircuitBreakerStats {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CircuitBreakerStats", 4)?;
        state.serialize_field("state", &self.state)?;
        state.serialize_field("failure_count", &self.failure_count)?;
        state.serialize_field("failure_threshold", &self.failure_threshold)?;
        state.serialize_field(
            "last_failure_time",
            &self.last_failure_time.map(|t| t.elapsed().as_secs()),
        )?;
        state.end()
    }
}

impl serde::Serialize for CircuitState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            CircuitState::Closed => serializer.serialize_str("closed"),
            CircuitState::Open => serializer.serialize_str("open"),
            CircuitState::HalfOpen => serializer.serialize_str("half_open"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_states() {
        let mut config = RoutingConfig::default();
        config.circuit_breaker_enabled = true;
        config.circuit_breaker_failure_threshold = 3.0; // Set as float, will be cast to u32
        config.circuit_breaker_timeout_seconds = 60;

        let service = CircuitBreakerService::new(config);

        // Initially closed
        assert!(service.can_execute("test-service"));
        assert_eq!(service.get_state("test-service"), CircuitState::Closed);

        // Record failures
        service.record_failure("test-service");
        service.record_failure("test-service");
        assert!(service.can_execute("test-service")); // Still closed

        service.record_failure("test-service");
        assert!(!service.can_execute("test-service")); // Now open
        assert_eq!(service.get_state("test-service"), CircuitState::Open);

        // Record success should close it
        service.record_success("test-service");
        assert!(service.can_execute("test-service"));
        assert_eq!(service.get_state("test-service"), CircuitState::Closed);
    }
}

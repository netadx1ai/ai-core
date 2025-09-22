// AI-CORE Test Data Generator Module
// Intelligent test data generation with AI-enhanced patterns
// Backend Agent Implementation - T2.2

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use faker_rand::en_us::{
    addresses::{CityName, StateName, StreetName, ZipCode},
    company::CompanyName,
    internet::{DomainName, Email, Password, Username},
    names::{FirstName, LastName},
    phone_numbers::PhoneNumber,
};
use rand::{prelude::*, thread_rng, Rng};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::database::DatabaseManager;
use crate::models::*;

// ============================================================================
// Test Data Generator - AI-Enhanced Data Generation Service
// ============================================================================

pub struct DataGenerator {
    database: Arc<DatabaseManager>,
    generation_jobs: Arc<RwLock<HashMap<Uuid, GenerationJob>>>,
    templates: Arc<RwLock<HashMap<String, DataTemplate>>>,
}

#[derive(Debug, Clone)]
struct GenerationJob {
    id: Uuid,
    request: GenerateDataRequest,
    status: GenerationStatus,
    progress: u32,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    generated_count: i32,
    output_urls: Vec<String>,
}

#[derive(Debug, Clone)]
enum GenerationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
struct DataTemplate {
    name: String,
    data_type: DataType,
    fields: HashMap<String, FieldTemplate>,
    relationships: Vec<DataRelationship>,
    business_rules: Vec<BusinessRule>,
}

#[derive(Debug, Clone)]
struct FieldTemplate {
    field_type: String,
    generator: GeneratorType,
    constraints: Option<FieldConstraint>,
    samples: Vec<Value>,
}

#[derive(Debug, Clone)]
enum GeneratorType {
    Random,
    Sequential,
    Pattern(String),
    Lookup(Vec<String>),
    Formula(String),
    AI(String), // AI-generated based on context
}

impl DataGenerator {
    pub async fn new(database: Arc<DatabaseManager>) -> Result<Self> {
        info!("Initializing DataGenerator with AI-enhanced capabilities");

        let generator = Self {
            database,
            generation_jobs: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize default templates
        generator.initialize_default_templates().await?;

        info!("DataGenerator initialized successfully");
        Ok(generator)
    }

    // ========================================================================
    // Public API Methods
    // ========================================================================

    pub async fn generate_data(&self, request: GenerateDataRequest) -> Result<DataGenerationResponse> {
        let generation_id = Uuid::new_v4();
        let now = Utc::now();

        debug!("Starting data generation: {} - {:?}", generation_id, request.data_generation.data_type);

        // Validate request
        self.validate_generation_request(&request).await?;

        // Create generation job
        let job = GenerationJob {
            id: generation_id,
            request: request.clone(),
            status: GenerationStatus::Pending,
            progress: 0,
            created_at: now,
            completed_at: None,
            error_message: None,
            generated_count: 0,
            output_urls: Vec::new(),
        };

        // Store job
        {
            let mut jobs = self.generation_jobs.write().await;
            jobs.insert(generation_id, job);
        }

        // Start generation in background
        let generator = self.clone();
        tokio::spawn(async move {
            if let Err(e) = generator.execute_generation(generation_id).await {
                error!("Data generation failed: {}", e);
                generator.mark_generation_failed(generation_id, e.to_string()).await;
            }
        });

        let estimated_completion = now + chrono::Duration::seconds(
            self.estimate_generation_time(&request.data_generation).await as i64
        );

        Ok(DataGenerationResponse {
            generation_id,
            status: "pending".to_string(),
            estimated_completion_time: estimated_completion,
            progress_url: format!("/api/generate-data/{}/status", generation_id),
            generated_count: 0,
            total_count: request.data_generation.count,
            data_urls: Vec::new(),
        })
    }

    pub async fn get_generation_status(&self, generation_id: Uuid) -> Result<DataGenerationResponse> {
        let jobs = self.generation_jobs.read().await;
        let job = jobs.get(&generation_id)
            .ok_or_else(|| anyhow!("Generation job not found"))?;

        Ok(DataGenerationResponse {
            generation_id,
            status: format!("{:?}", job.status).to_lowercase(),
            estimated_completion_time: job.completed_at.unwrap_or(job.created_at + chrono::Duration::seconds(300)),
            progress_url: format!("/api/generate-data/{}/status", generation_id),
            generated_count: job.generated_count,
            total_count: job.request.data_generation.count,
            data_urls: job.output_urls.clone(),
        })
    }

    // ========================================================================
    // Data Generation Implementation
    // ========================================================================

    async fn execute_generation(&self, generation_id: Uuid) -> Result<()> {
        info!("Executing data generation: {}", generation_id);

        // Update status to running
        self.update_job_status(generation_id, GenerationStatus::Running, 0).await;

        let job = {
            let jobs = self.generation_jobs.read().await;
            jobs.get(&generation_id).cloned()
                .ok_or_else(|| anyhow!("Generation job not found"))?
        };

        match job.request.data_generation.data_type {
            DataType::Users => self.generate_users(&job).await?,
            DataType::Workflows => self.generate_workflows(&job).await?,
            DataType::TestCases => self.generate_test_cases(&job).await?,
            DataType::Organizations => self.generate_organizations(&job).await?,
            DataType::Projects => self.generate_projects(&job).await?,
            DataType::Documents => self.generate_documents(&job).await?,
            DataType::Events => self.generate_events(&job).await?,
            DataType::Metrics => self.generate_metrics(&job).await?,
            DataType::Logs => self.generate_logs(&job).await?,
            DataType::Custom(ref custom_type) => self.generate_custom_data(&job, custom_type).await?,
        }

        // Mark as completed
        self.mark_generation_completed(generation_id).await;

        info!("Data generation completed: {}", generation_id);
        Ok(())
    }

    async fn generate_users(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} test users", job.request.data_generation.count);

        let batch_size = 100;
        let total_count = job.request.data_generation.count;
        let mut generated_count = 0;

        for batch_start in (0..total_count).step_by(batch_size) {
            let batch_end = std::cmp::min(batch_start + batch_size as i32, total_count);
            let batch_count = batch_end - batch_start;

            let mut batch_users = Vec::with_capacity(batch_count as usize);

            for i in 0..batch_count {
                let user = self.generate_test_user(&job.request.target_environment).await?;
                batch_users.push(user);
                generated_count += 1;

                // Update progress
                if i % 10 == 0 {
                    let progress = ((generated_count as f32 / total_count as f32) * 100.0) as u32;
                    self.update_job_progress(job.id, progress, generated_count).await;
                }
            }

            // Store batch in database
            for user in batch_users {
                let create_request = CreateTestUserRequest {
                    username: user.username,
                    email: user.email,
                    password: "GeneratedPassword123!".to_string(),
                    first_name: user.first_name,
                    last_name: user.last_name,
                    role: user.role,
                    permissions: user.permissions,
                    metadata: Some(user.metadata),
                    test_environment: user.test_environment,
                    ttl_hours: Some(72), // 3 days default
                };

                self.database.create_test_user(create_request).await?;
            }

            // Small delay to prevent overwhelming the database
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    async fn generate_workflows(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} test workflows", job.request.data_generation.count);

        let workflow_templates = vec![
            ("Data Processing Pipeline", "Automated data ingestion and processing"),
            ("User Onboarding Flow", "Complete user registration and verification"),
            ("Invoice Generation", "Automated invoice creation and delivery"),
            ("Content Approval Process", "Multi-stage content review and approval"),
            ("Customer Support Ticket", "Help desk ticket management system"),
            ("Marketing Campaign", "Email campaign management and tracking"),
            ("Inventory Management", "Stock level monitoring and reordering"),
            ("Employee Onboarding", "New hire process automation"),
            ("Quality Assurance", "Testing and quality control workflow"),
            ("Financial Reporting", "Automated financial data aggregation"),
        ];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let template = workflow_templates.choose(&mut rng).unwrap();
            let workflow_id = Uuid::new_v4();

            let workflow_definition = serde_json::json!({
                "version": "1.0",
                "triggers": [
                    {
                        "type": "manual",
                        "name": "Start Process"
                    }
                ],
                "steps": self.generate_workflow_steps(&mut rng),
                "variables": self.generate_workflow_variables(&mut rng),
                "error_handling": {
                    "retry_attempts": rng.gen_range(1..4),
                    "timeout_minutes": rng.gen_range(5..60),
                    "fallback_action": "notify_admin"
                }
            });

            let input_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "priority": {"type": "string", "enum": ["low", "medium", "high"]},
                    "department": {"type": "string"},
                    "requester_id": {"type": "string"},
                    "data": {"type": "object"}
                },
                "required": ["priority", "department", "requester_id"]
            });

            let output_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string", "enum": ["completed", "failed", "cancelled"]},
                    "result": {"type": "object"},
                    "execution_time_ms": {"type": "integer"},
                    "error_message": {"type": "string"}
                },
                "required": ["status", "execution_time_ms"]
            });

            // Store workflow (in a real implementation, you'd have a workflows table)
            debug!("Generated workflow: {} - {}", template.0, workflow_id);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_test_cases(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} test cases", job.request.data_generation.count);

        let test_categories = vec![
            "Authentication", "Authorization", "Data Validation", "API Integration",
            "User Interface", "Performance", "Security", "Error Handling",
            "Business Logic", "Workflow Execution", "Data Processing", "Reporting",
        ];

        let assertion_types = vec![
            AssertionType::Equals, AssertionType::NotEquals, AssertionType::Contains,
            AssertionType::GreaterThan, AssertionType::LessThan, AssertionType::IsNotNull,
        ];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let category = test_categories.choose(&mut rng).unwrap();
            let test_case_id = Uuid::new_v4();

            let test_case = TestCase {
                id: test_case_id,
                name: format!("{} Test Case {}", category, rng.gen_range(1000..9999)),
                description: Some(format!("Automated test case for {} functionality", category)),
                input_data: self.generate_test_input_data(&mut rng),
                expected_output: self.generate_expected_output(&mut rng),
                assertions: self.generate_test_assertions(&assertion_types, &mut rng),
                setup_steps: vec![
                    "Initialize test environment".to_string(),
                    "Prepare test data".to_string(),
                    "Configure system settings".to_string(),
                ],
                cleanup_steps: vec![
                    "Clean up test data".to_string(),
                    "Reset system state".to_string(),
                    "Archive test results".to_string(),
                ],
                timeout_seconds: rng.gen_range(30..300),
                retry_count: rng.gen_range(0..3),
            };

            debug!("Generated test case: {} - {}", test_case.name, test_case_id);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_organizations(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} organizations", job.request.data_generation.count);

        let industry_types = vec![
            "Technology", "Healthcare", "Finance", "Manufacturing", "Retail",
            "Education", "Government", "Non-profit", "Consulting", "Media",
        ];

        let company_sizes = vec!["Startup", "Small", "Medium", "Large", "Enterprise"];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let org_id = Uuid::new_v4();
            let company_name: String = CompanyName.fake(&mut rng);
            let industry = industry_types.choose(&mut rng).unwrap();
            let size = company_sizes.choose(&mut rng).unwrap();

            let organization = serde_json::json!({
                "id": org_id,
                "name": company_name,
                "industry": industry,
                "size": size,
                "employees": rng.gen_range(10..10000),
                "founded_year": rng.gen_range(1950..2024),
                "headquarters": {
                    "city": CityName.fake::<String>(&mut rng),
                    "state": StateName.fake::<String>(&mut rng),
                    "country": "USA"
                },
                "contact": {
                    "email": format!("info@{}.com", company_name.to_lowercase().replace(" ", "")),
                    "phone": PhoneNumber.fake::<String>(&mut rng)
                },
                "metadata": {
                    "test_organization": true,
                    "generated_at": Utc::now(),
                    "generator_version": "1.0"
                }
            });

            debug!("Generated organization: {} - {}", company_name, org_id);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_projects(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} projects", job.request.data_generation.count);

        let project_types = vec![
            "Web Application", "Mobile App", "API Service", "Data Pipeline",
            "Machine Learning", "DevOps Infrastructure", "Security Audit",
            "Database Migration", "System Integration", "Performance Optimization",
        ];

        let project_statuses = vec!["Planning", "In Progress", "Testing", "Deployment", "Completed", "On Hold"];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let project_id = Uuid::new_v4();
            let project_type = project_types.choose(&mut rng).unwrap();
            let status = project_statuses.choose(&mut rng).unwrap();

            let project = serde_json::json!({
                "id": project_id,
                "name": format!("{} Project {}", project_type, rng.gen_range(1000..9999)),
                "description": format!("Test project for {} development and testing", project_type),
                "type": project_type,
                "status": status,
                "priority": ["Low", "Medium", "High", "Critical"].choose(&mut rng).unwrap(),
                "budget": rng.gen_range(10000.0..1000000.0),
                "timeline": {
                    "start_date": Utc::now() - chrono::Duration::days(rng.gen_range(1..365)),
                    "end_date": Utc::now() + chrono::Duration::days(rng.gen_range(30..365)),
                    "estimated_hours": rng.gen_range(100..5000)
                },
                "team": {
                    "lead_id": Uuid::new_v4(),
                    "member_count": rng.gen_range(3..15),
                    "skills_required": ["Development", "Testing", "Design", "DevOps"]
                },
                "metadata": {
                    "test_project": true,
                    "environment": job.request.target_environment,
                    "generated_at": Utc::now()
                }
            });

            debug!("Generated project: {} - {}", project["name"], project_id);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_documents(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} documents", job.request.data_generation.count);

        let document_types = vec![
            "User Manual", "API Documentation", "Test Plan", "Requirements Specification",
            "Design Document", "Meeting Notes", "Project Report", "Technical Specification",
            "User Guide", "Installation Instructions", "Troubleshooting Guide", "FAQ",
        ];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let doc_id = Uuid::new_v4();
            let doc_type = document_types.choose(&mut rng).unwrap();

            let document = serde_json::json!({
                "id": doc_id,
                "title": format!("{} v{}.{}", doc_type, rng.gen_range(1..5), rng.gen_range(0..10)),
                "type": doc_type,
                "content": format!("This is a generated {} for testing purposes. It contains sample content that would typically be found in this type of document.", doc_type),
                "author": {
                    "name": format!("{} {}", FirstName.fake::<String>(&mut rng), LastName.fake::<String>(&mut rng)),
                    "email": Email.fake::<String>(&mut rng)
                },
                "version": format!("{}.{}.{}", rng.gen_range(1..5), rng.gen_range(0..10), rng.gen_range(0..100)),
                "status": ["Draft", "Review", "Approved", "Published", "Archived"].choose(&mut rng).unwrap(),
                "tags": ["test", "generated", "documentation"],
                "created_at": Utc::now() - chrono::Duration::days(rng.gen_range(1..365)),
                "updated_at": Utc::now() - chrono::Duration::days(rng.gen_range(0..30)),
                "word_count": rng.gen_range(500..5000),
                "metadata": {
                    "test_document": true,
                    "environment": job.request.target_environment
                }
            });

            debug!("Generated document: {} - {}", document["title"], doc_id);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_events(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} events", job.request.data_generation.count);

        let event_types = vec![
            "user.login", "user.logout", "user.created", "user.updated", "user.deleted",
            "workflow.started", "workflow.completed", "workflow.failed",
            "api.request", "api.error", "system.startup", "system.shutdown",
            "data.imported", "data.exported", "backup.created", "backup.restored",
        ];

        let severity_levels = vec!["info", "warning", "error", "critical"];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let event_id = Uuid::new_v4();
            let event_type = event_types.choose(&mut rng).unwrap();
            let severity = severity_levels.choose(&mut rng).unwrap();

            let event = serde_json::json!({
                "id": event_id,
                "type": event_type,
                "severity": severity,
                "timestamp": Utc::now() - chrono::Duration::seconds(rng.gen_range(0..86400)), // Last 24 hours
                "source": format!("service-{}", rng.gen_range(1..10)),
                "user_id": if event_type.starts_with("user.") { Some(Uuid::new_v4()) } else { None },
                "session_id": Uuid::new_v4(),
                "ip_address": format!("{}.{}.{}.{}",
                    rng.gen_range(1..255), rng.gen_range(1..255),
                    rng.gen_range(1..255), rng.gen_range(1..255)),
                "user_agent": "Mozilla/5.0 (TestBot/1.0)",
                "details": {
                    "message": format!("Generated test event for {}", event_type),
                    "duration_ms": rng.gen_range(1..5000),
                    "status_code": if event_type.starts_with("api.") { Some(rng.gen_range(200..500)) } else { None },
                    "error_code": if severity == "error" || severity == "critical" {
                        Some(format!("ERR_{}", rng.gen_range(1000..9999)))
                    } else { None }
                },
                "metadata": {
                    "test_event": true,
                    "environment": job.request.target_environment,
                    "correlation_id": Uuid::new_v4()
                }
            });

            debug!("Generated event: {} - {}", event_type, event_id);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_metrics(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} metrics", job.request.data_generation.count);

        let metric_names = vec![
            "cpu_usage_percent", "memory_usage_percent", "disk_usage_percent",
            "network_bytes_in", "network_bytes_out", "response_time_ms",
            "requests_per_second", "error_rate_percent", "active_connections",
            "queue_length", "cache_hit_rate", "database_connections",
        ];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let metric_name = metric_names.choose(&mut rng).unwrap();
            let timestamp = Utc::now() - chrono::Duration::seconds(rng.gen_range(0..3600)); // Last hour

            let value = match metric_name.as_ref() {
                "cpu_usage_percent" | "memory_usage_percent" | "disk_usage_percent" => rng.gen_range(0.0..100.0),
                "network_bytes_in" | "network_bytes_out" => rng.gen_range(1000.0..1000000.0),
                "response_time_ms" => rng.gen_range(10.0..2000.0),
                "requests_per_second" => rng.gen_range(1.0..1000.0),
                "error_rate_percent" => rng.gen_range(0.0..10.0),
                "active_connections" => rng.gen_range(1.0..500.0),
                "queue_length" => rng.gen_range(0.0..100.0),
                "cache_hit_rate" => rng.gen_range(70.0..99.0),
                "database_connections" => rng.gen_range(1.0..50.0),
                _ => rng.gen_range(0.0..1000.0),
            };

            let metric = serde_json::json!({
                "name": metric_name,
                "value": value,
                "timestamp": timestamp,
                "unit": self.get_metric_unit(metric_name),
                "tags": {
                    "service": format!("service-{}", rng.gen_range(1..5)),
                    "environment": job.request.target_environment,
                    "host": format!("host-{}", rng.gen_range(1..10)),
                    "region": ["us-east-1", "us-west-2", "eu-west-1"].choose(&mut rng).unwrap()
                },
                "metadata": {
                    "test_metric": true,
                    "generator_version": "1.0"
                }
            });

            debug!("Generated metric: {} = {} at {}", metric_name, value, timestamp);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_logs(&self, job: &GenerationJob) -> Result<()> {
        debug!("Generating {} log entries", job.request.data_generation.count);

        let log_levels = vec!["DEBUG", "INFO", "WARN", "ERROR", "FATAL"];
        let services = vec![
            "api-gateway", "user-service", "auth-service", "workflow-engine",
            "data-processor", "notification-service", "file-storage", "analytics",
        ];

        let log_messages = vec![
            "User authentication successful",
            "Processing workflow step",
            "Database connection established",
            "File uploaded successfully",
            "Cache miss for key",
            "API request processed",
            "Background job completed",
            "Configuration loaded",
            "Health check passed",
            "Metric collection complete",
        ];

        let mut rng = thread_rng();

        for i in 0..job.request.data_generation.count {
            let level = log_levels.choose(&mut rng).unwrap();
            let service = services.choose(&mut rng).unwrap();
            let message = log_messages.choose(&mut rng).unwrap();
            let timestamp = Utc::now() - chrono::Duration::seconds(rng.gen_range(0..7200)); // Last 2 hours

            let log_entry = serde_json::json!({
                "timestamp": timestamp,
                "level": level,
                "service": service,
                "message": message,
                "request_id": Uuid::new_v4(),
                "user_id": if rng.gen_bool(0.7) { Some(Uuid::new_v4()) } else { None },
                "session_id": if rng.gen_bool(0.8) { Some(Uuid::new_v4()) } else { None },
                "duration_ms": rng.gen_range(1..1000),
                "details": {
                    "method": ["GET", "POST", "PUT", "DELETE"].choose(&mut rng).unwrap(),
                    "path": format!("/api/v1/{}", ["users", "workflows", "data", "health"].choose(&mut rng).unwrap()),
                    "status_code": if level == "ERROR" { rng.gen_range(400..500) } else { rng.gen_range(200..300) },
                    "response_size": rng.gen_range(100..10000)
                },
                "metadata": {
                    "test_log": true,
                    "environment": job.request.target_environment,
                    "host": format!("host-{}", rng.gen_range(1..5)),
                    "version": "1.0.0"
                }
            });

            debug!("Generated log: {} - {} - {}", level, service, message);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    async fn generate_custom_data(&self, job: &GenerationJob, custom_type: &str) -> Result<()> {
        debug!("Generating {} custom data items of type: {}", job.request.data_generation.count, custom_type);

        // This would be extended based on custom requirements
        for i in 0..job.request.data_generation.count {
            let custom_data = serde_json::json!({
                "id": Uuid::new_v4(),
                "type": custom_type,
                "data": {
                    "generated": true,
                    "timestamp": Utc::now(),
                    "environment": job.request.target_environment
                }
            });

            debug!("Generated custom data: {} - {}", custom_type, custom_data["id"]);

            // Update progress
            if i % 10 == 0 {
                let progress = ((i as f32 / job.request.data_generation.count as f32) * 100.0) as u32;
                self.update_job_progress(job.id, progress, i).await;
            }
        }

        Ok(())
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    async fn generate_test_user(&self, environment: &str) -> Result<TestUser> {
        let mut rng = thread_rng();

        let first_name: String = FirstName.fake(&mut rng);
        let last_name: String = LastName.fake(&mut rng);
        let username = format!("{}_{}", first_name.to_lowercase(), rng.gen_range(1000..9999));
        let email = format!("{}@test-{}.com", username, environment);

        let roles = vec![
            UserRole::User, UserRole::Viewer, UserRole::Developer,
            UserRole::Tester, UserRole::Manager
        ];

        let permissions = match roles.choose(&mut rng).unwrap() {
            UserRole::Admin => vec!["*".to_string()],
            UserRole::Manager => vec!["read".to_string(), "write".to_string(), "manage".to_string()],
            UserRole::Developer => vec!["read".to_string(), "write".to_string(), "deploy".to_string()],
            UserRole::Tester => vec!["read".to_string(), "test".to_string()],
            UserRole::User => vec!["read".to_string()],
            UserRole::Viewer => vec!["read".to_string()],
            UserRole::Guest => vec!["limited_read".to_string()],
        };

        Ok(TestUser {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash: "hashed_password".to_string(),
            first_name: Some(first_name),
            last_name: Some(last_name),
            role: roles.choose(&mut rng).unwrap().clone(),
            permissions,
            metadata: serde_json::json!({
                "generated": true,
                "generator_version": "1.0",
                "created_by": "test-data-generator"
            }),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
            test_environment: environment.to_string(),
            cleanup_after: Some(Utc::now() + chrono::Duration::hours(72)),
        })
    }

    fn generate_workflow_steps(&self, rng: &mut ThreadRng) -> Vec<Value> {
        let step_count = rng.gen_range(3..8);
        let mut steps = Vec::new();

        for i in 0..step_count {
            steps.push(serde_json::json!({
                "id": i + 1,
                "name": format!("Step {}", i + 1),
                "type": ["condition", "action", "parallel", "loop"].choose(rng).unwrap(),
                "timeout_minutes": rng.gen_range(5..30),
                "retry_policy": {
                    "max_attempts": rng.gen_range(1..4),
                    "delay_seconds": rng.gen_range(5..60)
                }
            }));
        }

        steps
    }

    fn generate_workflow_variables(&self, rng: &mut ThreadRng) -> HashMap<String, Value> {
        let mut variables = HashMap::new();

        variables.insert("priority".to_string(), serde_json::json!("medium"));
        variables.insert("timeout".to_string(), serde_json::json!(rng.gen_range(300..3600)));
        variables.insert("retry_count".to_string(), serde_json::json!(rng.gen_range(1..5)));
        variables.insert("debug_mode".to_string(), serde_json::json!(rng.gen_bool(0.2)));

        variables
    }

    fn generate_test_input_data(&self, rng: &mut ThreadRng) -> Value {
        serde_json::json!({
            "test_parameter_1": format!("value_{}", rng.gen_range(1000..9999)),
            "test_parameter_2": rng.gen_range(1..100),
            "test_parameter_3": rng.gen_bool(0.5),
            "test_data": {
                "nested_value": format!("nested_{}", rng.gen_range(100..999)),
                "array_data": vec![1, 2, 3, 4, 5]
            }
        })
    }

    fn generate_expected_output(&self, rng: &mut ThreadRng) -> Value {
        serde_json::json!({
            "status": "success",
            "result_code": rng.gen_range(200..300),
            "message": "Operation completed successfully",
            "data": {
                "processed": true,
                "count": rng.gen_range(1..50),
                "timestamp": Utc::now()
            }
        })
    }

    fn generate_test_assertions(&self, assertion_types: &[AssertionType], rng: &mut ThreadRng) -> Vec<TestAssertion> {
        let count = rng.gen_range(2..6);
        let mut assertions = Vec::new();

        for _ in 0..count {
            assertions.push(TestAssertion {
                field_path: "result.status".to_string(),
                assertion_type: assertion_types.choose(rng).unwrap().clone(),
                expected_value: serde_json::json!("success"),
                tolerance: None,
            });
        }

        assertions
    }

    fn get_metric_unit(&self, metric_name: &str) -> String {
        match metric_name {
            name if name.contains("percent") => "percent".to_string(),
            name if name.contains("bytes") => "bytes".to_string(),
            name if name.contains("time_ms") => "milliseconds".to_string(),
            name if name.contains("per_second") => "per_second".to_string(),
            name if name.contains("connections") => "count".to_string(),
            _ => "unit".to_string(),
        }
    }

    async fn estimate_generation_time(&self, request: &DataGenerationRequest) -> u32 {
        // Estimate based on data type and count
        let base_time_per_item = match request.data_type {
            DataType::Users => 0.1,
            DataType::Workflows => 0.3,
            DataType::TestCases => 0.2,
            DataType::Organizations => 0.15,
            DataType::Projects => 0.2,
            DataType::Documents => 0.25,
            DataType::Events => 0.05,
            DataType::Metrics => 0.02,
            DataType::Logs => 0.01,
            DataType::Custom(_) => 0.1,
        };

        let estimated_seconds = (request.count as f32 * base_time_per_item) + 10.0; // Add 10s overhead
        estimated_seconds as u32
    }

    async fn validate_generation_request(&self, request: &GenerateDataRequest) -> Result<()> {
        if request.data_generation.count <= 0 || request.data_generation.count > 100000 {
            return Err(anyhow!("Invalid count: must be between 1 and 100000"));
        }

        if request.target_environment.is_empty() {
            return Err(anyhow!("Target environment cannot be empty"));
        }

        Ok(())
    }

    async fn initialize_default_templates(&self) -> Result<()> {
        debug!("Initializing default data generation templates");

        // Add default templates for common data types
        let mut templates = self.templates.write().await;

        templates.insert("user".to_string(), DataTemplate {
            name: "Default User Template".to_string(),
            data_type: DataType::Users,
            fields: HashMap::new(),
            relationships: Vec::new(),
            business_rules: Vec::new(),
        });

        templates.insert("workflow".to_string(), DataTemplate {
            name: "Default Workflow Template".to_string(),
            data_type: DataType::Workflows,
            fields: HashMap::new(),
            relationships: Vec::new(),
            business_rules: Vec::new(),
        });

        info!("Default templates initialized");
        Ok(())
    }

    async fn update_job_status(&self, job_id: Uuid, status: GenerationStatus, progress: u32) {
        if let Ok(mut jobs) = self.generation_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = status;
                job.progress = progress;
            }
        }
    }

    async fn update_job_progress(&self, job_id: Uuid, progress: u32, generated_count: i32) {
        if let Ok(mut jobs) = self.generation_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&job_id) {
                job.progress = progress;
                job.generated_count = generated_count;
            }
        }
    }

    async fn mark_generation_completed(&self, job_id: Uuid) {
        if let Ok(mut jobs) = self.generation_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = GenerationStatus::Completed;
                job.progress = 100;
                job.completed_at = Some(Utc::now());
            }
        }
    }

    async fn mark_generation_failed(&self, job_id: Uuid, error_message: String) {
        if let Ok(mut jobs) = self.generation_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = GenerationStatus::Failed;
                job.error_message = Some(error_message);
                job.completed_at = Some(Utc::now());
            }
        }
    }
}

// ============================================================================
// Clone implementation for shared usage
// ============================================================================

impl Clone for DataGenerator {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            generation_jobs: self.generation_jobs.clone(),
            templates: self.templates.clone(),
        }
    }
}

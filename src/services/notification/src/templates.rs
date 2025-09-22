//! Template management module for notification service
//!
//! This module provides template management functionality including:
//! - Template CRUD operations
//! - Template rendering with Handlebars
//! - Template caching
//! - Variable validation
//! - Multi-language support

use crate::config::TemplateConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::*;

use chrono::Utc;
use handlebars::Handlebars;
use mongodb::{
    bson::{doc, DateTime as BsonDateTime},
    options::FindOptions,
    Collection, Database,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Template manager for handling notification templates
#[derive(Clone)]
pub struct TemplateManager {
    config: TemplateConfig,
    handlebars: Arc<RwLock<Handlebars<'static>>>,
    template_cache: Arc<RwLock<HashMap<String, NotificationTemplate>>>,
    mongo: Option<Database>,
}

impl TemplateManager {
    /// Create a new template manager
    pub async fn new(config: &TemplateConfig) -> Result<Self> {
        info!("Initializing template manager");

        let mut handlebars = Handlebars::new();

        // Configure Handlebars
        handlebars.set_strict_mode(false);
        handlebars.set_dev_mode(cfg!(debug_assertions));

        // Register built-in helpers
        Self::register_helpers(&mut handlebars)?;

        let manager = Self {
            config: config.clone(),
            handlebars: Arc::new(RwLock::new(handlebars)),
            template_cache: Arc::new(RwLock::new(HashMap::new())),
            mongo: None, // TODO: Initialize MongoDB connection
        };

        // Load default templates
        manager.load_default_templates().await?;

        info!("Template manager initialized successfully");
        Ok(manager)
    }

    /// Create a new notification template
    pub async fn create_template(
        &self,
        request: CreateTemplateRequest,
    ) -> Result<NotificationTemplate> {
        info!("Creating notification template: {}", request.name);

        let template_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Validate template syntax
        self.validate_template_syntax(&request.subject_template, &request.content_template)?;

        let template = NotificationTemplate {
            id: template_id,
            name: request.name,
            description: request.description,
            notification_type: request.notification_type,
            channels: request.channels,
            subject_template: request.subject_template,
            content_template: request.content_template,
            variables: request.variables,
            created_at: now,
            updated_at: now,
            is_active: true,
        };

        // Store in database
        self.store_template(&template).await?;

        // Register with Handlebars
        self.register_template(&template).await?;

        // Update cache
        if self.config.cache_enabled {
            let mut cache = self.template_cache.write().await;
            cache.insert(template.id.clone(), template.clone());
        }

        info!("Template created successfully: {}", template.id);
        Ok(template)
    }

    /// Update an existing notification template
    pub async fn update_template(
        &self,
        id: &str,
        request: UpdateTemplateRequest,
    ) -> Result<NotificationTemplate> {
        info!("Updating notification template: {}", id);

        let mut template = self
            .get_template(id)
            .await?
            .ok_or_else(|| NotificationError::not_found("template"))?;

        // Update fields if provided
        if let Some(name) = request.name {
            template.name = name;
        }
        if let Some(description) = request.description {
            template.description = Some(description);
        }
        if let Some(channels) = request.channels {
            template.channels = channels;
        }
        if let Some(subject_template) = request.subject_template {
            template.subject_template = subject_template;
        }
        if let Some(content_template) = request.content_template {
            template.content_template = content_template;
        }
        if let Some(variables) = request.variables {
            template.variables = variables;
        }
        if let Some(is_active) = request.is_active {
            template.is_active = is_active;
        }

        template.updated_at = Utc::now();

        // Validate updated template syntax
        self.validate_template_syntax(&template.subject_template, &template.content_template)?;

        // Update in database
        self.update_template_record(&template).await?;

        // Re-register with Handlebars
        self.register_template(&template).await?;

        // Update cache
        if self.config.cache_enabled {
            let mut cache = self.template_cache.write().await;
            cache.insert(template.id.clone(), template.clone());
        }

        info!("Template updated successfully: {}", template.id);
        Ok(template)
    }

    /// Get template by ID
    pub async fn get_template(&self, id: &str) -> Result<Option<NotificationTemplate>> {
        // Check cache first
        if self.config.cache_enabled {
            let cache = self.template_cache.read().await;
            if let Some(template) = cache.get(id) {
                return Ok(Some(template.clone()));
            }
        }

        // Fallback to database
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationTemplate> = mongo.collection("templates");
            let filter = doc! { "id": id };

            match collection.find_one(filter, None).await {
                Ok(result) => {
                    if let Some(ref template) = result {
                        // Update cache
                        if self.config.cache_enabled {
                            let mut cache = self.template_cache.write().await;
                            cache.insert(template.id.clone(), template.clone());
                        }
                    }
                    Ok(result)
                }
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    /// List templates with optional filtering
    pub async fn list_templates(
        &self,
        notification_type: Option<NotificationType>,
        is_active: Option<bool>,
    ) -> Result<Vec<NotificationTemplate>> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationTemplate> = mongo.collection("templates");
            let mut filter = doc! {};

            if let Some(nt) = notification_type {
                filter.insert("notification_type", nt.to_string());
            }
            if let Some(active) = is_active {
                filter.insert("is_active", active);
            }

            let options = FindOptions::builder()
                .sort(doc! { "created_at": -1 })
                .build();

            match collection.find(filter, options).await {
                Ok(mut cursor) => {
                    let mut templates = Vec::new();
                    while cursor
                        .advance()
                        .await
                        .map_err(|e| NotificationError::database(e.to_string()))?
                    {
                        let template = cursor
                            .deserialize_current()
                            .map_err(|e| NotificationError::database(e.to_string()))?;
                        templates.push(template);
                    }
                    Ok(templates)
                }
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// Delete a template
    pub async fn delete_template(&self, id: &str) -> Result<bool> {
        info!("Deleting notification template: {}", id);

        // Remove from cache
        if self.config.cache_enabled {
            let mut cache = self.template_cache.write().await;
            cache.remove(id);
        }

        // Unregister from Handlebars
        {
            let mut handlebars = self.handlebars.write().await;
            handlebars.unregister_template(&format!("subject_{}", id));
            handlebars.unregister_template(&format!("content_{}", id));
        }

        // Remove from database
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationTemplate> = mongo.collection("templates");
            let filter = doc! { "id": id };

            match collection.delete_one(filter, None).await {
                Ok(result) => {
                    info!("Template deleted successfully: {}", id);
                    Ok(result.deleted_count > 0)
                }
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(false)
        }
    }

    /// Render notification content using template
    pub async fn render_notification(
        &self,
        template_id: &str,
        data: &Option<serde_json::Value>,
    ) -> Result<(String, String)> {
        let template = self
            .get_template(template_id)
            .await?
            .ok_or_else(|| NotificationError::not_found("template"))?;

        if !template.is_active {
            return Err(NotificationError::template("Template is not active"));
        }

        let context = data.clone().unwrap_or_else(|| serde_json::json!({}));

        // Render subject
        let subject = {
            let handlebars = self.handlebars.read().await;
            handlebars
                .render(&format!("subject_{}", template_id), &context)
                .map_err(|e| NotificationError::template(format!("Subject render error: {}", e)))?
        };

        // Render content
        let content = {
            let handlebars = self.handlebars.read().await;
            handlebars
                .render(&format!("content_{}", template_id), &context)
                .map_err(|e| NotificationError::template(format!("Content render error: {}", e)))?
        };

        Ok((subject, content))
    }

    /// Validate template variables against provided data
    pub fn validate_template_data(
        &self,
        template: &NotificationTemplate,
        data: &serde_json::Value,
    ) -> Result<()> {
        for variable in &template.variables {
            if variable.is_required {
                if !data
                    .as_object()
                    .unwrap_or(&serde_json::Map::new())
                    .contains_key(&variable.name)
                {
                    return Err(NotificationError::validation(
                        &variable.name,
                        "Required variable is missing",
                    ));
                }
            }
        }
        Ok(())
    }

    // Private helper methods

    fn register_helpers(handlebars: &mut Handlebars) -> Result<()> {
        // Register date formatting helper
        handlebars.register_helper(
            "date",
            Box::new(
                |h: &handlebars::Helper,
                 _: &Handlebars,
                 _: &handlebars::Context,
                 _rc: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param = h.param(0).unwrap().value();
                    let format = h
                        .param(1)
                        .map(|v| v.value().as_str().unwrap_or("%Y-%m-%d"))
                        .unwrap_or("%Y-%m-%d");

                    if let Some(date_str) = param.as_str() {
                        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(date_str) {
                            out.write(&datetime.format(format).to_string())?;
                        } else {
                            out.write(date_str)?;
                        }
                    } else {
                        out.write(&param.to_string())?;
                    }
                    Ok(())
                },
            ),
        );

        // Register uppercase helper
        handlebars.register_helper(
            "uppercase",
            Box::new(
                |h: &handlebars::Helper,
                 _: &Handlebars,
                 _: &handlebars::Context,
                 _rc: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param = h.param(0).unwrap();
                    out.write(&param.value().as_str().unwrap_or("").to_uppercase())?;
                    Ok(())
                },
            ),
        );

        Ok(())
    }

    async fn load_default_templates(&self) -> Result<()> {
        info!("Loading default notification templates");

        let default_templates = vec![
            (
                NotificationType::WorkflowCompleted,
                "Workflow Completed",
                "Workflow {{workflow_name}} completed successfully",
                "Your workflow '{{workflow_name}}' has completed successfully at {{date completion_time '%Y-%m-%d %H:%M:%S'}}.",
            ),
            (
                NotificationType::WorkflowFailed,
                "Workflow Failed",
                "Workflow {{workflow_name}} failed",
                "Your workflow '{{workflow_name}}' failed with error: {{error_message}}. Please check the logs for more details.",
            ),
            (
                NotificationType::SystemAlert,
                "System Alert",
                "System Alert: {{alert_title}}",
                "{{alert_message}}",
            ),
        ];

        for (notification_type, name, subject, content) in default_templates {
            let request = CreateTemplateRequest {
                name: name.to_string(),
                description: Some(format!("Default template for {:?}", notification_type)),
                notification_type: notification_type.clone(),
                channels: vec![
                    NotificationChannel::Email,
                    NotificationChannel::Push,
                    NotificationChannel::Websocket,
                ],
                subject_template: subject.to_string(),
                content_template: content.to_string(),
                variables: vec![TemplateVariable {
                    name: "workflow_name".to_string(),
                    variable_type: VariableType::String,
                    description: Some("Name of the workflow".to_string()),
                    default_value: Some("Unknown Workflow".to_string()),
                    is_required: false,
                }],
            };

            if let Err(e) = self.create_template(request).await {
                warn!(
                    "Failed to create default template for {:?}: {}",
                    notification_type, e
                );
            }
        }

        Ok(())
    }

    fn validate_template_syntax(&self, subject: &str, content: &str) -> Result<()> {
        // Test compile templates
        let mut temp_handlebars = Handlebars::new();

        temp_handlebars
            .register_template_string("test_subject", subject)
            .map_err(|e| {
                NotificationError::template(format!("Subject template syntax error: {}", e))
            })?;

        temp_handlebars
            .register_template_string("test_content", content)
            .map_err(|e| {
                NotificationError::template(format!("Content template syntax error: {}", e))
            })?;

        Ok(())
    }

    async fn register_template(&self, template: &NotificationTemplate) -> Result<()> {
        let mut handlebars = self.handlebars.write().await;

        handlebars
            .register_template_string(
                &format!("subject_{}", template.id),
                &template.subject_template,
            )
            .map_err(|e| {
                NotificationError::template(format!("Failed to register subject template: {}", e))
            })?;

        handlebars
            .register_template_string(
                &format!("content_{}", template.id),
                &template.content_template,
            )
            .map_err(|e| {
                NotificationError::template(format!("Failed to register content template: {}", e))
            })?;

        Ok(())
    }

    async fn store_template(&self, template: &NotificationTemplate) -> Result<()> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationTemplate> = mongo.collection("templates");
            match collection.insert_one(template, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(()) // Skip if no database
        }
    }

    async fn update_template_record(&self, template: &NotificationTemplate) -> Result<()> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationTemplate> = mongo.collection("templates");
            let filter = doc! { "id": &template.id };
            let update = doc! {
                "$set": {
                    "name": &template.name,
                    "description": &template.description,
                    "channels": mongodb::bson::to_bson(&template.channels).unwrap(),
                    "subject_template": &template.subject_template,
                    "content_template": &template.content_template,
                    "variables": mongodb::bson::to_bson(&template.variables).unwrap(),
                    "is_active": template.is_active,
                    "updated_at": BsonDateTime::from_system_time(template.updated_at.into())
                }
            };

            match collection.update_one(filter, update, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TemplateConfig;

    fn create_test_config() -> TemplateConfig {
        TemplateConfig {
            cache_enabled: true,
            cache_size: 100,
            cache_ttl_seconds: 3600,
            template_directory: None,
            default_locale: "en".to_string(),
            supported_locales: vec!["en".to_string()],
        }
    }

    #[tokio::test]
    async fn test_template_manager_creation() {
        let config = create_test_config();
        let manager = TemplateManager::new(&config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_template_creation() {
        let config = create_test_config();
        let manager = TemplateManager::new(&config).await.unwrap();

        let request = CreateTemplateRequest {
            name: "Test Template".to_string(),
            description: Some("Test description".to_string()),
            notification_type: NotificationType::WorkflowCompleted,
            channels: vec![NotificationChannel::Email],
            subject_template: "Test: {{title}}".to_string(),
            content_template: "Hello {{name}}, your {{type}} is ready!".to_string(),
            variables: vec![TemplateVariable {
                name: "title".to_string(),
                variable_type: VariableType::String,
                description: None,
                default_value: None,
                is_required: true,
            }],
        };

        let result = manager.create_template(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_rendering() {
        let config = create_test_config();
        let manager = TemplateManager::new(&config).await.unwrap();

        let request = CreateTemplateRequest {
            name: "Test Template".to_string(),
            description: Some("Test description".to_string()),
            notification_type: NotificationType::WorkflowCompleted,
            channels: vec![NotificationChannel::Email],
            subject_template: "Test: {{title}}".to_string(),
            content_template: "Hello {{name}}, your {{type}} is ready!".to_string(),
            variables: vec![],
        };

        let template = manager.create_template(request).await.unwrap();

        let data = serde_json::json!({
            "title": "Notification",
            "name": "John",
            "type": "workflow"
        });

        let result = manager.render_notification(&template.id, &Some(data)).await;
        assert!(result.is_ok());

        let (subject, content) = result.unwrap();
        assert_eq!(subject, "Test: Notification");
        assert_eq!(content, "Hello John, your workflow is ready!");
    }

    #[tokio::test]
    async fn test_template_syntax_validation() {
        let config = create_test_config();
        let manager = TemplateManager::new(&config).await.unwrap();

        // Valid template
        let result = manager.validate_template_syntax("Hello {{name}}", "Content: {{message}}");
        assert!(result.is_ok());

        // Invalid template
        let result = manager.validate_template_syntax("Hello {{name", "Content: {{message}}");
        assert!(result.is_err());
    }
}

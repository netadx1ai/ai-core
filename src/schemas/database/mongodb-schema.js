// MongoDB Schema: Flexible Document Storage
// Handles 25% of data: Content, campaigns, client configurations, workflow details
// Optimized for flexible schemas and rapid prototypying

// Database: automation_platform
use("automation_platform");

// ============================================================================
// CONTENT MANAGEMENT COLLECTIONS
// ============================================================================

// Content items - Generated content and media
db.createCollection("content_items", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["content_type", "title", "status", "created_by", "created_at"],
      properties: {
        _id: { bsonType: "objectId" },
        content_id: { bsonType: "string" },
        workflow_id: { bsonType: "string" },
        campaign_id: { bsonType: "string" },
        content_type: {
          bsonType: "string",
          enum: [
            "blog",
            "social_post",
            "image",
            "video",
            "infographic",
            "carousel",
            "story",
            "reel",
            "email",
            "landing_page",
          ],
        },
        title: { bsonType: "string", maxLength: 500 },
        body: { bsonType: "string" },
        summary: { bsonType: "string", maxLength: 1000 },
        media_urls: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
        hashtags: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
        mentions: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
        call_to_action: { bsonType: "string" },
        target_platforms: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
        seo_metadata: {
          bsonType: "object",
          properties: {
            meta_title: { bsonType: "string", maxLength: 60 },
            meta_description: { bsonType: "string", maxLength: 160 },
            keywords: {
              bsonType: "array",
              items: { bsonType: "string" },
            },
            canonical_url: { bsonType: "string" },
          },
        },
        performance_data: {
          bsonType: "object",
          properties: {
            impressions: { bsonType: "long" },
            clicks: { bsonType: "long" },
            shares: { bsonType: "long" },
            likes: { bsonType: "long" },
            comments: { bsonType: "long" },
            engagement_rate: { bsonType: "double" },
            conversion_rate: { bsonType: "double" },
          },
        },
        status: {
          bsonType: "string",
          enum: [
            "draft",
            "pending_approval",
            "approved",
            "published",
            "archived",
          ],
        },
        approval_workflow: {
          bsonType: "object",
          properties: {
            required: { bsonType: "bool" },
            approved_by: { bsonType: "string" },
            approved_at: { bsonType: "date" },
            comments: { bsonType: "string" },
          },
        },
        created_by: { bsonType: "string" },
        created_at: { bsonType: "date" },
        updated_at: { bsonType: "date" },
        published_at: { bsonType: "date" },
        expires_at: { bsonType: "date" },
        tags: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
        version: { bsonType: "int" },
        language: { bsonType: "string" },
        localization: {
          bsonType: "object",
        },
      },
    },
  },
});

// Content templates for reuse
db.createCollection("content_templates", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["name", "content_type", "template", "created_by"],
      properties: {
        _id: { bsonType: "objectId" },
        template_id: { bsonType: "string" },
        name: { bsonType: "string", maxLength: 200 },
        description: { bsonType: "string", maxLength: 1000 },
        content_type: {
          bsonType: "string",
          enum: ["blog", "social_post", "email", "landing_page", "ad_copy"],
        },
        template: {
          bsonType: "object",
          properties: {
            structure: { bsonType: "string" },
            placeholders: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  name: { bsonType: "string" },
                  type: { bsonType: "string" },
                  required: { bsonType: "bool" },
                  default_value: { bsonType: "string" },
                },
              },
            },
            style_guide: {
              bsonType: "object",
              properties: {
                tone: { bsonType: "string" },
                voice: { bsonType: "string" },
                brand_guidelines: { bsonType: "object" },
              },
            },
          },
        },
        usage_count: { bsonType: "long" },
        success_rate: { bsonType: "double" },
        category: { bsonType: "string" },
        tags: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
        is_public: { bsonType: "bool" },
        created_by: { bsonType: "string" },
        created_at: { bsonType: "date" },
        updated_at: { bsonType: "date" },
      },
    },
  },
});

// ============================================================================
// CAMPAIGN MANAGEMENT COLLECTIONS
// ============================================================================

// Marketing campaigns with flexible structures
db.createCollection("campaigns", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["name", "campaign_type", "status", "created_by"],
      properties: {
        _id: { bsonType: "objectId" },
        campaign_id: { bsonType: "string" },
        name: { bsonType: "string", maxLength: 200 },
        description: { bsonType: "string" },
        campaign_type: {
          bsonType: "string",
          enum: [
            "product_launch",
            "brand_awareness",
            "lead_generation",
            "retargeting",
            "seasonal",
            "event_promotion",
          ],
        },
        status: {
          bsonType: "string",
          enum: [
            "draft",
            "planned",
            "active",
            "paused",
            "completed",
            "cancelled",
          ],
        },
        objectives: {
          bsonType: "array",
          items: {
            bsonType: "object",
            properties: {
              metric: { bsonType: "string" },
              target_value: { bsonType: "double" },
              current_value: { bsonType: "double" },
            },
          },
        },
        target_audience: {
          bsonType: "object",
          properties: {
            demographics: {
              bsonType: "object",
              properties: {
                age_range: { bsonType: "string" },
                gender: { bsonType: "string" },
                location: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
                interests: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
              },
            },
            behavioral: {
              bsonType: "object",
              properties: {
                purchase_behavior: { bsonType: "string" },
                engagement_level: { bsonType: "string" },
                device_usage: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
              },
            },
          },
        },
        channels: {
          bsonType: "array",
          items: {
            bsonType: "object",
            properties: {
              platform: { bsonType: "string" },
              budget_allocation: { bsonType: "double" },
              content_count: { bsonType: "int" },
              posting_schedule: {
                bsonType: "object",
                properties: {
                  frequency: { bsonType: "string" },
                  optimal_times: {
                    bsonType: "array",
                    items: { bsonType: "string" },
                  },
                  timezone: { bsonType: "string" },
                },
              },
            },
          },
        },
        budget: {
          bsonType: "object",
          properties: {
            total_budget: { bsonType: "double" },
            spent_amount: { bsonType: "double" },
            currency: { bsonType: "string" },
            cost_per_channel: {
              bsonType: "object",
            },
          },
        },
        timeline: {
          bsonType: "object",
          properties: {
            start_date: { bsonType: "date" },
            end_date: { bsonType: "date" },
            milestones: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  name: { bsonType: "string" },
                  date: { bsonType: "date" },
                  completed: { bsonType: "bool" },
                },
              },
            },
          },
        },
        content_calendar: {
          bsonType: "array",
          items: {
            bsonType: "object",
            properties: {
              content_id: { bsonType: "string" },
              platform: { bsonType: "string" },
              scheduled_time: { bsonType: "date" },
              status: { bsonType: "string" },
            },
          },
        },
        performance_metrics: {
          bsonType: "object",
          properties: {
            total_reach: { bsonType: "long" },
            total_impressions: { bsonType: "long" },
            total_clicks: { bsonType: "long" },
            total_conversions: { bsonType: "long" },
            roi: { bsonType: "double" },
            cost_per_acquisition: { bsonType: "double" },
            engagement_rate: { bsonType: "double" },
          },
        },
        created_by: { bsonType: "string" },
        created_at: { bsonType: "date" },
        updated_at: { bsonType: "date" },
        tags: {
          bsonType: "array",
          items: { bsonType: "string" },
        },
      },
    },
  },
});

// Scheduled posts for content publishing
db.createCollection("scheduled_posts", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["content_id", "platform", "scheduled_time", "status"],
      properties: {
        _id: { bsonType: "objectId" },
        post_id: { bsonType: "string" },
        campaign_id: { bsonType: "string" },
        content_id: { bsonType: "string" },
        platform: { bsonType: "string" },
        platform_specific_config: {
          bsonType: "object",
        },
        scheduled_time: { bsonType: "date" },
        published_time: { bsonType: "date" },
        status: {
          bsonType: "string",
          enum: ["scheduled", "publishing", "published", "failed", "cancelled"],
        },
        retry_count: { bsonType: "int" },
        max_retries: { bsonType: "int" },
        error_message: { bsonType: "string" },
        platform_response: {
          bsonType: "object",
          properties: {
            post_id: { bsonType: "string" },
            post_url: { bsonType: "string" },
            platform_metrics: { bsonType: "object" },
          },
        },
        approval_status: {
          bsonType: "string",
          enum: ["pending", "approved", "rejected", "not_required"],
        },
        approved_by: { bsonType: "string" },
        approved_at: { bsonType: "date" },
        created_at: { bsonType: "date" },
        updated_at: { bsonType: "date" },
      },
    },
  },
});

// ============================================================================
// CLIENT CONFIGURATION COLLECTIONS
// ============================================================================

// Client configurations for federation
db.createCollection("client_configs", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["client_id", "client_name", "status"],
      properties: {
        _id: { bsonType: "objectId" },
        client_id: { bsonType: "string" },
        client_name: { bsonType: "string" },
        client_type: {
          bsonType: "string",
          enum: ["enterprise", "partner", "developer", "individual"],
        },
        status: {
          bsonType: "string",
          enum: ["active", "inactive", "suspended", "pending_approval"],
        },
        configuration: {
          bsonType: "object",
          properties: {
            api_endpoints: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  name: { bsonType: "string" },
                  url: { bsonType: "string" },
                  auth_type: { bsonType: "string" },
                  capabilities: {
                    bsonType: "array",
                    items: { bsonType: "string" },
                  },
                },
              },
            },
            mcp_servers: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  server_id: { bsonType: "string" },
                  name: { bsonType: "string" },
                  endpoint: { bsonType: "string" },
                  tools: {
                    bsonType: "array",
                    items: {
                      bsonType: "object",
                      properties: {
                        name: { bsonType: "string" },
                        cost_per_call: { bsonType: "double" },
                        schema: { bsonType: "object" },
                      },
                    },
                  },
                  health_check_url: { bsonType: "string" },
                  success_rate: { bsonType: "double" },
                },
              },
            },
            data_residency: {
              bsonType: "object",
              properties: {
                requirements: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
                allowed_regions: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
              },
            },
            compliance: {
              bsonType: "object",
              properties: {
                requirements: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
                certifications: {
                  bsonType: "array",
                  items: { bsonType: "string" },
                },
              },
            },
            rate_limits: {
              bsonType: "object",
              properties: {
                requests_per_minute: { bsonType: "int" },
                requests_per_hour: { bsonType: "int" },
                concurrent_workflows: { bsonType: "int" },
              },
            },
          },
        },
        preferences: {
          bsonType: "object",
          properties: {
            preferred_providers: {
              bsonType: "array",
              items: { bsonType: "string" },
            },
            cost_optimization: { bsonType: "bool" },
            quality_threshold: { bsonType: "double" },
            notification_settings: {
              bsonType: "object",
              properties: {
                webhook_url: { bsonType: "string" },
                email_notifications: { bsonType: "bool" },
                slack_integration: {
                  bsonType: "object",
                  properties: {
                    enabled: { bsonType: "bool" },
                    webhook_url: { bsonType: "string" },
                    channel: { bsonType: "string" },
                  },
                },
              },
            },
          },
        },
        statistics: {
          bsonType: "object",
          properties: {
            total_requests: { bsonType: "long" },
            successful_workflows: { bsonType: "long" },
            failed_workflows: { bsonType: "long" },
            average_cost_per_workflow: { bsonType: "double" },
            total_spend: { bsonType: "double" },
            last_activity: { bsonType: "date" },
          },
        },
        created_at: { bsonType: "date" },
        updated_at: { bsonType: "date" },
        created_by: { bsonType: "string" },
      },
    },
  },
});

// ============================================================================
// WORKFLOW DETAILS COLLECTIONS
// ============================================================================

// Extended workflow configurations and templates
db.createCollection("workflow_configurations", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["workflow_id", "configuration_type"],
      properties: {
        _id: { bsonType: "objectId" },
        workflow_id: { bsonType: "string" },
        configuration_type: {
          bsonType: "string",
          enum: ["template", "instance", "blueprint"],
        },
        configuration_data: {
          bsonType: "object",
          properties: {
            steps: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  step_id: { bsonType: "string" },
                  provider_config: {
                    bsonType: "object",
                    properties: {
                      primary_provider: { bsonType: "string" },
                      fallback_providers: {
                        bsonType: "array",
                        items: { bsonType: "string" },
                      },
                      cost_threshold: { bsonType: "double" },
                      quality_threshold: { bsonType: "double" },
                    },
                  },
                  parameters: { bsonType: "object" },
                  conditional_logic: {
                    bsonType: "object",
                    properties: {
                      conditions: {
                        bsonType: "array",
                        items: {
                          bsonType: "object",
                          properties: {
                            field: { bsonType: "string" },
                            operator: { bsonType: "string" },
                            value: {},
                            action: { bsonType: "string" },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
            global_settings: {
              bsonType: "object",
              properties: {
                timeout_strategy: { bsonType: "string" },
                error_handling: { bsonType: "string" },
                cost_control: {
                  bsonType: "object",
                  properties: {
                    max_cost: { bsonType: "double" },
                    cost_alerts: {
                      bsonType: "array",
                      items: { bsonType: "double" },
                    },
                  },
                },
              },
            },
          },
        },
        usage_statistics: {
          bsonType: "object",
          properties: {
            execution_count: { bsonType: "long" },
            success_rate: { bsonType: "double" },
            average_cost: { bsonType: "double" },
            average_duration: { bsonType: "long" },
          },
        },
        created_at: { bsonType: "date" },
        updated_at: { bsonType: "date" },
      },
    },
  },
});

// ============================================================================
// ANALYTICS AND INSIGHTS COLLECTIONS
// ============================================================================

// User behavior and usage patterns
db.createCollection("user_insights", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["user_id", "insight_type"],
      properties: {
        _id: { bsonType: "objectId" },
        user_id: { bsonType: "string" },
        insight_type: {
          bsonType: "string",
          enum: ["usage_pattern", "preference", "behavior", "recommendation"],
        },
        data: {
          bsonType: "object",
          properties: {
            most_used_workflows: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  workflow_type: { bsonType: "string" },
                  usage_count: { bsonType: "int" },
                  success_rate: { bsonType: "double" },
                },
              },
            },
            preferred_providers: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  provider_id: { bsonType: "string" },
                  usage_percentage: { bsonType: "double" },
                  satisfaction_score: { bsonType: "double" },
                },
              },
            },
            cost_patterns: {
              bsonType: "object",
              properties: {
                average_spend_per_month: { bsonType: "double" },
                spending_trend: { bsonType: "string" },
                cost_distribution: { bsonType: "object" },
              },
            },
            recommendations: {
              bsonType: "array",
              items: {
                bsonType: "object",
                properties: {
                  type: { bsonType: "string" },
                  suggestion: { bsonType: "string" },
                  potential_savings: { bsonType: "double" },
                  confidence_score: { bsonType: "double" },
                },
              },
            },
          },
        },
        generated_at: { bsonType: "date" },
        expires_at: { bsonType: "date" },
      },
    },
  },
});

// ============================================================================
// INDEXES FOR PERFORMANCE OPTIMIZATION
// ============================================================================

// Content Items Indexes
db.content_items.createIndex({ content_id: 1 }, { unique: true });
db.content_items.createIndex({ workflow_id: 1 });
db.content_items.createIndex({ campaign_id: 1 });
db.content_items.createIndex({ content_type: 1 });
db.content_items.createIndex({ status: 1 });
db.content_items.createIndex({ created_by: 1 });
db.content_items.createIndex({ created_at: -1 });
db.content_items.createIndex({ target_platforms: 1 });
db.content_items.createIndex({ tags: 1 });
db.content_items.createIndex({ published_at: -1 });

// Content Templates Indexes
db.content_templates.createIndex({ template_id: 1 }, { unique: true });
db.content_templates.createIndex({ content_type: 1 });
db.content_templates.createIndex({ category: 1 });
db.content_templates.createIndex({ is_public: 1 });
db.content_templates.createIndex({ usage_count: -1 });
db.content_templates.createIndex({ success_rate: -1 });

// Campaigns Indexes
db.campaigns.createIndex({ campaign_id: 1 }, { unique: true });
db.campaigns.createIndex({ campaign_type: 1 });
db.campaigns.createIndex({ status: 1 });
db.campaigns.createIndex({ created_by: 1 });
db.campaigns.createIndex({ "timeline.start_date": 1 });
db.campaigns.createIndex({ "timeline.end_date": 1 });
db.campaigns.createIndex({ "channels.platform": 1 });

// Scheduled Posts Indexes
db.scheduled_posts.createIndex({ post_id: 1 }, { unique: true });
db.scheduled_posts.createIndex({ campaign_id: 1 });
db.scheduled_posts.createIndex({ content_id: 1 });
db.scheduled_posts.createIndex({ platform: 1 });
db.scheduled_posts.createIndex({ scheduled_time: 1 });
db.scheduled_posts.createIndex({ status: 1 });
db.scheduled_posts.createIndex({ approval_status: 1 });

// Client Configs Indexes
db.client_configs.createIndex({ client_id: 1 }, { unique: true });
db.client_configs.createIndex({ client_type: 1 });
db.client_configs.createIndex({ status: 1 });
db.client_configs.createIndex({ "statistics.last_activity": -1 });

// Workflow Configurations Indexes
db.workflow_configurations.createIndex({ workflow_id: 1 });
db.workflow_configurations.createIndex({ configuration_type: 1 });
db.workflow_configurations.createIndex({
  "usage_statistics.execution_count": -1,
});
db.workflow_configurations.createIndex({ "usage_statistics.success_rate": -1 });

// User Insights Indexes
db.user_insights.createIndex({ user_id: 1 });
db.user_insights.createIndex({ insight_type: 1 });
db.user_insights.createIndex({ generated_at: -1 });
db.user_insights.createIndex({ expires_at: 1 }, { expireAfterSeconds: 0 });

// ============================================================================
// SAMPLE DATA FOR DEVELOPMENT
// ============================================================================

// Insert sample content template
db.content_templates.insertOne({
  template_id: "blog_post_template_001",
  name: "Standard Blog Post Template",
  description: "A versatile template for creating engaging blog posts",
  content_type: "blog",
  template: {
    structure:
      "{{introduction}}\n\n{{main_content}}\n\n{{conclusion}}\n\n{{call_to_action}}",
    placeholders: [
      {
        name: "introduction",
        type: "text",
        required: true,
        default_value: "Hook the reader with an interesting opening",
      },
      {
        name: "main_content",
        type: "rich_text",
        required: true,
        default_value: "Main body content with key points and insights",
      },
      {
        name: "conclusion",
        type: "text",
        required: true,
        default_value: "Summarize key takeaways",
      },
      {
        name: "call_to_action",
        type: "text",
        required: false,
        default_value: "What do you want readers to do next?",
      },
    ],
    style_guide: {
      tone: "professional",
      voice: "authoritative",
      brand_guidelines: {
        use_active_voice: true,
        max_paragraph_length: 3,
        include_statistics: true,
      },
    },
  },
  usage_count: NumberLong(0),
  success_rate: 0.95,
  category: "marketing",
  tags: ["blog", "marketing", "content"],
  is_public: true,
  created_by: "system",
  created_at: new Date(),
  updated_at: new Date(),
});

print("MongoDB schema creation completed successfully!");
print(
  "Collections created: content_items, content_templates, campaigns, scheduled_posts, client_configs, workflow_configurations, user_insights",
);
print("Indexes created for optimal query performance");
print("Sample data inserted for development");

// MongoDB Schema: Flexible Document Storage
// Handles 25% of data: Content, campaigns, client configurations, workflow details
// Optimized for flexible schemas and rapid prototyping

// Database: automation_platform
use('automation_platform');

// ============================================================================
// CONTENT MANAGEMENT COLLECTIONS
// ============================================================================

// Content items - Generated content and media
db.createCollection('content_items', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['content_type', 'title', 'status', 'created_by', 'created_at'],
      properties: {
        _id: { bsonType: 'objectId' },
        content_id: { bsonType: 'string' },
        workflow_id: { bsonType: 'string' },
        campaign_id: { bsonType: 'string' },
        content_type: {
          bsonType: 'string',
          enum: ['blog', 'social_post', 'image', 'video', 'infographic', 'carousel', 'story', 'reel', 'email', 'landing_page']
        },
        title: { bsonType: 'string', maxLength: 500 },
        body: { bsonType: 'string' },
        summary: { bsonType: 'string', maxLength: 1000 },
        media_urls: {
          bsonType: 'array',
          items: { bsonType: 'string' }
        },
        hashtags: {
          bsonType: 'array',
          items: { bsonType: 'string' }
        },
        mentions: {
          bsonType: 'array',
          items: { bsonType: 'string' }
        },
        call_to_action: { bsonType: 'string' },
        target_platforms: {
          bsonType: 'array',
          items: { bsonType: 'string' }
        },
        seo_metadata: {
          bsonType: 'object',
          properties: {
            meta_title: { bsonType: 'string', maxLength: 60 },
            meta_description: { bsonType: 'string', maxLength: 160 },
            keywords: { bsonType: 'array', items: { bsonType: 'string' } },
            slug: { bsonType: 'string' },
            canonical_url: { bsonType: 'string' }
          }
        },
        performance_metrics: {
          bsonType: 'object',
          properties: {
            impressions: { bsonType: 'int' },
            reach: { bsonType: 'int' },
            clicks: { bsonType: 'int' },
            shares: { bsonType: 'int' },
            likes: { bsonType: 'int' },
            comments: { bsonType: 'int' },
            engagement_rate: { bsonType: 'double' },
            cost_per_engagement: { bsonType: 'double' }
          }
        },
        ai_metadata: {
          bsonType: 'object',
          properties: {
            model_used: { bsonType: 'string' },
            prompt_version: { bsonType: 'string' },
            generation_params: { bsonType: 'object' },
            quality_score: { bsonType: 'double', minimum: 0, maximum: 1 },
            confidence_score: { bsonType: 'double', minimum: 0, maximum: 1 },
            content_category: { bsonType: 'string' },
            sentiment: { bsonType: 'string', enum: ['positive', 'neutral', 'negative'] },
            readability_score: { bsonType: 'double' }
          }
        },
        status: {
          bsonType: 'string',
          enum: ['draft', 'pending_review', 'approved', 'published', 'archived', 'rejected']
        },
        approval_history: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              reviewer_id: { bsonType: 'string' },
              action: { bsonType: 'string', enum: ['approved', 'rejected', 'requested_changes'] },
              comments: { bsonType: 'string' },
              timestamp: { bsonType: 'date' }
            }
          }
        },
        version: { bsonType: 'int', minimum: 1 },
        parent_content_id: { bsonType: 'string' }, // For versioning
        language: { bsonType: 'string', default: 'en' },
        brand_guidelines: {
          bsonType: 'object',
          properties: {
            brand_voice: { bsonType: 'string' },
            tone: { bsonType: 'string' },
            style_guide_url: { bsonType: 'string' },
            compliance_checked: { bsonType: 'bool' }
          }
        },
        created_by: { bsonType: 'string' },
        created_at: { bsonType: 'date' },
        updated_at: { bsonType: 'date' },
        published_at: { bsonType: 'date' },
        expires_at: { bsonType: 'date' },
        tags: {
          bsonType: 'array',
          items: { bsonType: 'string' }
        },
        custom_fields: { bsonType: 'object' } // Flexible additional data
      }
    }
  }
});

// Content templates for rapid generation
db.createCollection('content_templates', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['name', 'content_type', 'template_body', 'created_by'],
      properties: {
        _id: { bsonType: 'objectId' },
        name: { bsonType: 'string' },
        description: { bsonType: 'string' },
        content_type: { bsonType: 'string' },
        category: { bsonType: 'string' },
        template_body: { bsonType: 'string' },
        placeholders: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              name: { bsonType: 'string' },
              type: { bsonType: 'string' },
              required: { bsonType: 'bool' },
              default_value: { bsonType: 'string' },
              description: { bsonType: 'string' }
            }
          }
        },
        target_platforms: { bsonType: 'array', items: { bsonType: 'string' } },
        usage_count: { bsonType: 'int', minimum: 0 },
        performance_score: { bsonType: 'double', minimum: 0, maximum: 10 },
        is_active: { bsonType: 'bool', default: true },
        created_by: { bsonType: 'string' },
        created_at: { bsonType: 'date' },
        updated_at: { bsonType: 'date' },
        tags: { bsonType: 'array', items: { bsonType: 'string' } }
      }
    }
  }
});

// ============================================================================
// CAMPAIGN MANAGEMENT COLLECTIONS
// ============================================================================

// Marketing campaigns
db.createCollection('campaigns', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['name', 'campaign_type', 'status', 'created_by', 'created_at'],
      properties: {
        _id: { bsonType: 'objectId' },
        campaign_id: { bsonType: 'string' },
        workflow_id: { bsonType: 'string' },
        name: { bsonType: 'string' },
        description: { bsonType: 'string' },
        campaign_type: {
          bsonType: 'string',
          enum: ['product_launch', 'brand_awareness', 'lead_generation', 'retargeting', 'seasonal', 'event_promotion']
        },
        status: {
          bsonType: 'string',
          enum: ['draft', 'scheduled', 'active', 'paused', 'completed', 'cancelled']
        },
        objectives: {
          bsonType: 'array',
          items: { bsonType: 'string' }
        },
        target_audience: {
          bsonType: 'object',
          properties: {
            demographics: {
              bsonType: 'object',
              properties: {
                age_range: { bsonType: 'string' },
                gender: { bsonType: 'string' },
                income_level: { bsonType: 'string' },
                education: { bsonType: 'string' },
                location: { bsonType: 'array', items: { bsonType: 'string' } }
              }
            },
            interests: { bsonType: 'array', items: { bsonType: 'string' } },
            behaviors: { bsonType: 'array', items: { bsonType: 'string' } },
            custom_audiences: { bsonType: 'array', items: { bsonType: 'string' } }
          }
        },
        channels: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              platform: { bsonType: 'string' },
              budget_allocation: { bsonType: 'double' },
              objectives: { bsonType: 'array', items: { bsonType: 'string' } },
              targeting: { bsonType: 'object' },
              creative_specs: { bsonType: 'object' }
            }
          }
        },
        budget: {
          bsonType: 'object',
          properties: {
            total_budget: { bsonType: 'double' },
            daily_budget: { bsonType: 'double' },
            spent_amount: { bsonType: 'double' },
            currency: { bsonType: 'string', default: 'USD' },
            budget_allocation: {
              bsonType: 'object',
              patternProperties: {
                "^[a-z_]+$": { bsonType: 'double' }
              }
            }
          }
        },
        timeline: {
          bsonType: 'object',
          properties: {
            start_date: { bsonType: 'date' },
            end_date: { bsonType: 'date' },
            launch_date: { bsonType: 'date' },
            milestones: {
              bsonType: 'array',
              items: {
                bsonType: 'object',
                properties: {
                  name: { bsonType: 'string' },
                  date: { bsonType: 'date' },
                  status: { bsonType: 'string' },
                  description: { bsonType: 'string' }
                }
              }
            }
          }
        },
        content_plan: {
          bsonType: 'object',
          properties: {
            content_calendar_id: { bsonType: 'string' },
            content_themes: { bsonType: 'array', items: { bsonType: 'string' } },
            posting_frequency: { bsonType: 'object' },
            content_mix: { bsonType: 'object' } // Percentage breakdown by content type
          }
        },
        kpis: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              metric: { bsonType: 'string' },
              target_value: { bsonType: 'double' },
              current_value: { bsonType: 'double' },
              unit: { bsonType: 'string' }
            }
          }
        },
        performance_data: {
          bsonType: 'object',
          properties: {
            impressions: { bsonType: 'long' },
            reach: { bsonType: 'long' },
            clicks: { bsonType: 'long' },
            conversions: { bsonType: 'long' },
            cost_per_click: { bsonType: 'double' },
            cost_per_conversion: { bsonType: 'double' },
            return_on_ad_spend: { bsonType: 'double' },
            engagement_rate: { bsonType: 'double' },
            last_updated: { bsonType: 'date' }
          }
        },
        automation_rules: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              rule_name: { bsonType: 'string' },
              condition: { bsonType: 'object' },
              action: { bsonType: 'object' },
              is_active: { bsonType: 'bool' }
            }
          }
        },
        created_by: { bsonType: 'string' },
        created_at: { bsonType: 'date' },
        updated_at: { bsonType: 'date' },
        tags: { bsonType: 'array', items: { bsonType: 'string' } },
        metadata: { bsonType: 'object' }
      }
    }
  }
});

// Scheduled content publishing
db.createCollection('scheduled_posts', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['content_id', 'platform', 'scheduled_time', 'status', 'created_by'],
      properties: {
        _id: { bsonType: 'objectId' },
        post_id: { bsonType: 'string' },
        content_id: { bsonType: 'string' },
        campaign_id: { bsonType: 'string' },
        workflow_id: { bsonType: 'string' },
        platform: { bsonType: 'string' },
        platform_specific_config: {
          bsonType: 'object',
          properties: {
            facebook: {
              bsonType: 'object',
              properties: {
                page_id: { bsonType: 'string' },
                post_type: { bsonType: 'string' },
                targeting: { bsonType: 'object' },
                boost_budget: { bsonType: 'double' }
              }
            },
            instagram: {
              bsonType: 'object',
              properties: {
                account_id: { bsonType: 'string' },
                post_type: { bsonType: 'string', enum: ['feed', 'story', 'reel'] },
                location_tag: { bsonType: 'string' },
                product_tags: { bsonType: 'array' }
              }
            },
            twitter: {
              bsonType: 'object',
              properties: {
                thread_sequence: { bsonType: 'int' },
                reply_to_tweet_id: { bsonType: 'string' },
                enable_replies: { bsonType: 'bool' }
              }
            },
            linkedin: {
              bsonType: 'object',
              properties: {
                visibility: { bsonType: 'string', enum: ['public', 'connections', 'logged-in'] },
                company_page: { bsonType: 'bool' }
              }
            }
          }
        },
        scheduled_time: { bsonType: 'date' },
        actual_publish_time: { bsonType: 'date' },
        status: {
          bsonType: 'string',
          enum: ['pending', 'scheduled', 'published', 'failed', 'cancelled', 'requires_approval']
        },
        approval_status: {
          bsonType: 'string',
          enum: ['pending', 'approved', 'rejected', 'changes_requested']
        },
        priority: {
          bsonType: 'string',
          enum: ['low', 'medium', 'high', 'urgent'],
          default: 'medium'
        },
        retry_config: {
          bsonType: 'object',
          properties: {
            max_retries: { bsonType: 'int', default: 3 },
            retry_count: { bsonType: 'int', default: 0 },
            retry_delay_minutes: { bsonType: 'int', default: 15 },
            last_retry_at: { bsonType: 'date' }
          }
        },
        publishing_result: {
          bsonType: 'object',
          properties: {
            platform_post_id: { bsonType: 'string' },
            platform_url: { bsonType: 'string' },
            success: { bsonType: 'bool' },
            error_message: { bsonType: 'string' },
            response_data: { bsonType: 'object' }
          }
        },
        analytics_tracking: {
          bsonType: 'object',
          properties: {
            utm_source: { bsonType: 'string' },
            utm_medium: { bsonType: 'string' },
            utm_campaign: { bsonType: 'string' },
            utm_content: { bsonType: 'string' },
            tracking_pixels: { bsonType: 'array', items: { bsonType: 'string' } }
          }
        },
        created_by: { bsonType: 'string' },
        created_at: { bsonType: 'date' },
        updated_at: { bsonType: 'date' },
        metadata: { bsonType: 'object' }
      }
    }
  }
});

// ============================================================================
// WORKFLOW AND PROCESS MANAGEMENT
// ============================================================================

// Detailed workflow information (supplements PostgreSQL basic data)
db.createCollection('workflow_details', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['workflow_id', 'user_id', 'workflow_type', 'status'],
      properties: {
        _id: { bsonType: 'objectId' },
        workflow_id: { bsonType: 'string' },
        user_id: { bsonType: 'string' },
        workflow_type: { bsonType: 'string' },
        original_request: { bsonType: 'string' },
        parsed_intent: {
          bsonType: 'object',
          properties: {
            primary_goal: { bsonType: 'string' },
            secondary_goals: { bsonType: 'array', items: { bsonType: 'string' } },
            complexity_score: { bsonType: 'double' },
            estimated_duration: { bsonType: 'int' },
            required_capabilities: { bsonType: 'array', items: { bsonType: 'string' } }
          }
        },
        execution_plan: {
          bsonType: 'object',
          properties: {
            steps: {
              bsonType: 'array',
              items: {
                bsonType: 'object',
                properties: {
                  step_id: { bsonType: 'string' },
                  step_name: { bsonType: 'string' },
                  step_type: { bsonType: 'string' },
                  provider: { bsonType: 'string' },
                  dependencies: { bsonType: 'array', items: { bsonType: 'string' } },
                  parameters: { bsonType: 'object' },
                  estimated_cost: { bsonType: 'double' },
                  estimated_duration: { bsonType: 'int' }
                }
              }
            },
            parallel_execution_groups: { bsonType: 'array' },
            critical_path: { bsonType: 'array', items: { bsonType: 'string' } }
          }
        },
        execution_history: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              step_id: { bsonType: 'string' },
              status: { bsonType: 'string' },
              start_time: { bsonType: 'date' },
              end_time: { bsonType: 'date' },
              provider_used: { bsonType: 'string' },
              actual_cost: { bsonType: 'double' },
              outputs: { bsonType: 'object' },
              error_details: { bsonType: 'object' },
              performance_metrics: { bsonType: 'object' }
            }
          }
        },
        status: { bsonType: 'string' },
        progress_percentage: { bsonType: 'int', minimum: 0, maximum: 100 },
        current_step: { bsonType: 'string' },
        federation_context: {
          bsonType: 'object',
          properties: {
            client_id: { bsonType: 'string' },
            client_capabilities_used: { bsonType: 'array' },
            cost_sharing: { bsonType: 'object' },
            data_residency_requirements: { bsonType: 'object' }
          }
        },
        optimization_data: {
          bsonType: 'object',
          properties: {
            provider_selection_reasons: { bsonType: 'object' },
            cost_optimizations_applied: { bsonType: 'array' },
            performance_improvements: { bsonType: 'array' },
            alternative_paths_considered: { bsonType: 'int' }
          }
        },
        quality_metrics: {
          bsonType: 'object',
          properties: {
            overall_quality_score: { bsonType: 'double', minimum: 0, maximum: 10 },
            user_satisfaction_rating: { bsonType: 'int', minimum: 1, maximum: 5 },
            completion_rate: { bsonType: 'double' },
            error_rate: { bsonType: 'double' },
            retry_count: { bsonType: 'int' }
          }
        },
        created_at: { bsonType: 'date' },
        updated_at: { bsonType: 'date' },
        completed_at: { bsonType: 'date' },
        metadata: { bsonType: 'object' }
      }
    }
  }
});

// ============================================================================
// CLIENT AND FEDERATION MANAGEMENT
// ============================================================================

// Client configurations and customizations
db.createCollection('client_configs', {
  validator: {
    $jsonSchema: {
      bsonType: 'object',
      required: ['client_id', 'client_name', 'status'],
      properties: {
        _id: { bsonType: 'objectId' },
        client_id: { bsonType: 'string' },
        client_name: { bsonType: 'string' },
        organization: { bsonType: 'string' },
        industry: { bsonType: 'string' },
        status: { bsonType: 'string', enum: ['active', 'inactive', 'suspended', 'trial'] },
        tier: { bsonType: 'string', enum: ['basic', 'professional', 'enterprise'] },
        configuration: {
          bsonType: 'object',
          properties: {
            branding: {
              bsonType: 'object',
              properties: {
                logo_url: { bsonType: 'string' },
                primary_color: { bsonType: 'string' },
                secondary_color: { bsonType: 'string' },
                custom_css: { bsonType: 'string' }
              }
            },
            default_settings: {
              bsonType: 'object',
              properties: {
                language: { bsonType: 'string', default: 'en' },
                timezone: { bsonType: 'string' },
                currency: { bsonType: 'string', default: 'USD' },
                date_format: { bsonType: 'string' },
                notification_preferences: { bsonType: 'object' }
              }
            },
            workflow_defaults: {
              bsonType: 'object',
              properties: {
                auto_approval: { bsonType: 'bool', default: false },
                default_providers: { bsonType: 'object' },
                cost_limits: { bsonType: 'object' },
                quality_thresholds: { bsonType: 'object' }
              }
            },
            integration_settings: {
              bsonType: 'object',
              properties: {
                webhook_endpoints: { bsonType: 'array' },
                api_rate_limits: { bsonType: 'object' },
                data_retention_policy: { bsonType: 'object' },
                security_settings: { bsonType: 'object' }
              }
            }
          }
        },
        mcp_servers: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              server_id: { bsonType: 'string' },
              server_name: { bsonType: 'string' },
              endpoint: { bsonType: 'string' },
              capabilities: { bsonType: 'array' },
              cost_model: { bsonType: 'object' },
              sla_requirements: { bsonType: 'object' },
              health_status: { bsonType: 'string' },
              last_health_check: { bsonType: 'date' }
            }
          }
        },
        usage_analytics: {
          bsonType: 'object',
          properties: {
            total_workflows: { bsonType: 'long' },
            total_cost: { bsonType: 'double' },
            average_workflow_duration: { bsonType: 'double' },
            success_rate: { bsonType: 'double' },
            preferred_providers: { bsonType: 'object' },
            monthly_usage_trends: { bsonType: 'array' }
          }
        },
        contacts: {
          bsonType: 'array',
          items: {
            bsonType: 'object',
            properties: {
              name: { bsonType: 'string' },
              email: { bsonType: 'string' },
              role: { bsonType: 'string' },
              phone: { bsonType: 'string' },
              is_primary: { bsonType: 'bool' }
            }
          }
        },
        billing_info: {
          bsonType: 'object',
          properties: {
            billing_email: { bsonType: 'string' },
            payment_method: { bsonType: 'string' },
            billing_address: { bsonType: 'object' },
            tax_id: { bsonType: 'string' },
            purchase_order_required: { bsonType: 'bool' }
          }
        },
        compliance: {
          bsonType: 'object',
          properties: {
            gdpr_compliant: { bsonType: 'bool' },
            hipaa_required: { bsonType: 'bool' },
            soc2_certified: { bsonType: 'bool' },
            data_residency_requirements: { bsonType: 'array' },
            audit_log_retention_days: { bsonType: 'int' }
          }
        },
        created_at: { bsonType: 'date' },
        updated_at: { bsonType: 'date' },
        last_activity: { bsonType: 'date' },
        metadata: { bsonType: 'object' }
      }
    }
  }
});

// ============================================================================
// INDEXES FOR PERFORMANCE OPTIMIZATION
// ============================================================================

// Content items indexes
db.content_items.createIndex({ 'content_id': 1 }, { unique: true });
db.content_items.createIndex({ 'workflow_id': 1 });
db.content_items.createIndex({ 'campaign_id': 1 });
db.content_items.createIndex({ 'created_by': 1, 'created_at': -1 });
db.content_items.createIndex({ 'content_type': 1, 'status': 1 });
db.content_items.createIndex({ 'target_platforms': 1 });
db.content_items.createIndex({ 'status': 1, 'published_at': -1 });
db.content_items.createIndex({ 'hashtags': 1 });
db.content_items.createIndex({ 'seo_metadata.keywords': 1 });
db.content_items.createIndex({ 'ai_metadata.quality_score': -1 });
db.content_items.createIndex({ 'created_at': -1 }); // For recent content queries
db.content_items.createIndex({ 'expires_at': 1 }); // For TTL cleanup

// Content templates indexes
db.content_templates.createIndex({ 'name': 1 });
db.content_templates.createIndex({ 'content_type': 1, 'category': 1 });
db.content_templates.createIndex({ 'target_platforms': 1 });
db.content_templates.createIndex({ 'performance_score': -1 });
db.content_templates.createIndex({ 'usage_count': -1 });
db.content_templates.createIndex({ 'is_active': 1 });

// Campaigns indexes
db.campaigns.createIndex({ 'campaign_id': 1 }, { unique: true });
db.campaigns.createIndex({ 'workflow_id': 1 });
db.campaigns.createIndex({ 'created_by': 1, 'created_at': -1 });
db.campaigns.createIndex({ 'status': 1 });
db.campaigns.createIndex({ 'campaign_type': 1 });
db.campaigns.createIndex({ 'timeline.start_date': 1, 'timeline.end_date': 1 });
db.campaigns.createIndex({ 'channels.platform': 1 });
db.campaigns.createIndex({ 'kpis.metric': 1 });

// Scheduled posts indexes
db.scheduled_posts.createIndex({ 'post_id': 1 }, { unique: true });
db.scheduled_posts.createIndex({ 'content_id': 1 });
db.scheduled_posts.createIndex({ 'campaign_id': 1 });
db.scheduled_posts.createIndex({ 'workflow_id': 1 });
db.scheduled_posts.createIndex({ 'platform': 1, 'scheduled_time': 1 });
db.scheduled_posts.createIndex({ 'status': 1, 'scheduled_time': 1 });
db.scheduled_posts.createIndex({ 'created_by': 1 });
db.scheduled_posts.createIndex({ 'scheduled_time': 1 }); // For scheduling queries
db.scheduled_posts.createIndex({ 'priority': 1, 'scheduled_time': 1 });
db.scheduled_posts.createIndex({ 'approval_status': 1 });

// Workflow details indexes
db.workflow_details.createIndex({ 'workflow_id': 1 }, { unique: true });
db.workflow_details.createIndex({ 'user_id': 1, 'created_at': -1 });
db.workflow_details.createIndex({ 'workflow_type': 1 });
db.workflow_details.createIndex({ 'status': 1 });
db.workflow_details.createIndex({ 'federation_context.client_id': 1 });
db.workflow_details.createIndex({ 'quality_metrics.overall_quality_score': -1 });
db.workflow_details.createIndex({ 'created_at': -1 });

// Client configs indexes
db.client_configs.createIndex({ 'client_id': 1 }, { unique: true });
db.client_configs.createIndex({ 'client_name': 1 });
db.client_configs.createIndex({ 'status': 1 });
db.client_configs.createIndex({ 'tier': 1 });
db.client_configs.createIndex({ 'industry': 1 });
db.client_configs.createIndex({ 'mcp_servers.server_id': 1 });
db.client_configs.createIndex({ 'last_activity': -1 });

// Compound indexes for common query patterns
db.content_items.createIndex({ 'created_by': 1, 'status': 1, 'created_at': -1 });
db.content_items.createIndex({ 'campaign_id': 1, 'content_type': 1, 'status': 1 });
db.scheduled_posts.createIndex({ 'platform': 1, 'status': 1, 'scheduled_time': 1 });
db.campaigns.createIndex({ 'created_by': 1, 'status': 1, 'timeline.start_date': -1 });

// Text indexes for search functionality
db.content_items.createIndex({
  'title': 'text',
  'body': 'text',
  'hashtags': 'text',
  'seo_metadata.keywords': 'text'
}, {
  weights: {
    'title': 10,
    'hashtags': 5,
    'seo_metadata.keywords': 3,
    'body': 1
  },
  name: 'content_search_index'
});

db.campaigns.createIndex({
  'name': 'text',
  'description': 'text',
  'objectives': 'text'
}, {
  weights: {
    'name': 10,
    'objectives': 5, 
    'description': 1
  },
  name: 'campaign_search_index'
});

// ============================================================================
// SAMPLE DATA AND UTILITY FUNCTIONS
// ============================================================================

// Sample content item
db.content_items.insertOne({
  content_id: 'content_001',
  workflow_id: 'workflow_123',
  campaign_id: 'campaign_abc',
  content_type: 'social_post',
  title: 'Exciting New Product Launch!',
  body: 'We are thrilled to announce the launch of our revolutionary new coffee blend. Made with sustainably sourced beans from Ethiopia and Colombia, this blend offers a perfect balance of boldness and smoothness. #CoffeeLovers #NewProduct #Sustainable',
  hashtags: ['CoffeeLovers', 'NewProduct', 'Sustainable', 'QualityCoffee'],
  call_to_action: 'Try it today and taste the difference!',
  target_platforms: ['facebook', 'instagram', 'twitter'],
  seo_metadata: {
    meta_title: 'New Coffee Blend Launch - Premium Quality',
    meta_description: 'Discover our new sustainable coffee blend with Ethiopian and Colombian beans.',
    keywords: ['coffee', 'sustainable', 'premium', 'new product', 'Ethiopian coffee'],
    slug: 'new-coffee-blend-launch'
  },
  ai_metadata: {
    model_used: 'gpt-4',
    prompt_version: 'v2.1',
    quality_score: 0.92,
    confidence_score: 0.88,
    content_category: 'product_announcement',
    sentiment: 'positive',
    readability_score: 8.5
  },
  status: 'approved',
  version: 1,
  language: 'en',
  created_by: 'user_456',
  created_at: new Date(),
  updated_at: new Date(),
  tags: ['product-launch', 'coffee', 'marketing']
});

// Utility functions
function getActiveContentByPlatform(platform) {
  return db.content_items.find({
    target_platforms: platform,
    status: { $in: ['approved', 'published'] }
  }).sort({ created_at: -1 });
}

function getCampaignPerformance(campaignId) {
  return db.campaigns.aggregate([
    { $match: { campaign_id: campaignId } },
    {
      $lookup: {
        from: 'content_items',
        localField: 'campaign_id',
        foreignField: 'campaign_id',
        as: 'content'
      }
    },
    {
      $lookup: {
        from: 'scheduled_posts',
        localField: 'campaign_id',
        foreignField: 'campaign_id',
        as: 'posts'
      }
    },
    {
      $project: {
        name: 1,
        status: 1,
        performance_data: 1,
        total_content: { $size: '$content' },
        total_posts: { $size: '$posts' },
        published_posts: {
          $size: {
            $filter: {
              input: '$posts',
              cond: { $eq: ['$$this.status', 'published'] }
            }
          }
        }
      }
    }
  ]);
}

// TTL indexes for automatic cleanup
db.content_items.createIndex({ 'expires_at': 1 }, { expireAfterSeconds: 0 });
db.scheduled_posts.createIndex({ 'created_at': 1 }, { expireAfterSeconds: 31536000 }); // 1 year

print('MongoDB schema initialization completed successfully');
print('Collections created: content_items, content_templates, campaigns, scheduled_posts, workflow_details, client_configs');
print('Indexes created for optimal query performance');
print('Validators configured for data integrity');
print('Sample data and utility functions added');
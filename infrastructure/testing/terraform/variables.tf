# AI-PLATFORM Testing Infrastructure - Terraform Variables
# Version: 1.0
# Created: 2025-01-11
# Status: ACTIVE - Implementation Phase
# Architect: architect_agent
# Classification: P0 Critical Path Foundation

# ===== GENERAL CONFIGURATION =====

variable "environment" {
  description = "Environment name (e.g., testing, staging, production)"
  type        = string
  default     = "testing"

  validation {
    condition = contains([
      "local",
      "development",
      "testing",
      "staging",
      "production"
    ], var.environment)
    error_message = "Environment must be one of: local, development, testing, staging, production."
  }
}

variable "aws_region" {
  description = "AWS region for testing infrastructure"
  type        = string
  default     = "us-west-2"

  validation {
    condition = can(regex("^[a-z]{2}-[a-z]+-[0-9]$", var.environment == "testing" ? var.aws_region : "us-west-2"))
    error_message = "AWS region must be in format like 'us-west-2'."
  }
}

variable "project_name" {
  description = "Name of the project"
  type        = string
  default     = "AI-PLATFORM"
}

variable "component_name" {
  description = "Component name for resource naming"
  type        = string
  default     = "testing"
}

# ===== EKS CLUSTER CONFIGURATION =====

variable "cluster_version" {
  description = "Kubernetes version for the EKS cluster"
  type        = string
  default     = "1.28"

  validation {
    condition = can(regex("^1\\.(2[6-9]|[3-9][0-9])$", var.cluster_version))
    error_message = "Cluster version must be 1.26 or higher."
  }
}

variable "cluster_endpoint_public_access" {
  description = "Enable public API server endpoint access"
  type        = bool
  default     = true
}

variable "cluster_endpoint_private_access" {
  description = "Enable private API server endpoint access"
  type        = bool
  default     = true
}

variable "cluster_endpoint_public_access_cidrs" {
  description = "List of CIDR blocks that can access the public API server endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

# ===== NODE GROUP CONFIGURATION =====

variable "node_groups" {
  description = "EKS node group configurations"
  type = map(object({
    instance_types = list(string)
    capacity_type  = string
    min_size      = number
    max_size      = number
    desired_size  = number
    disk_size     = number
    disk_type     = string
    ami_type      = string
    labels        = map(string)
    taints        = map(object({
      key    = string
      value  = string
      effect = string
    }))
  }))

  default = {
    general = {
      instance_types = ["t3.large", "t3.xlarge"]
      capacity_type  = "ON_DEMAND"
      min_size      = 1
      max_size      = 10
      desired_size  = 3
      disk_size     = 50
      disk_type     = "gp3"
      ami_type      = "AL2_x86_64"
      labels = {
        NodeGroup = "general"
        WorkloadType = "general"
      }
      taints = {}
    }

    performance = {
      instance_types = ["r5.xlarge", "r5.2xlarge"]
      capacity_type  = "SPOT"
      min_size      = 0
      max_size      = 5
      desired_size  = 1
      disk_size     = 100
      disk_type     = "gp3"
      ami_type      = "AL2_x86_64"
      labels = {
        NodeGroup = "performance"
        WorkloadType = "performance-testing"
      }
      taints = {
        dedicated = {
          key    = "performance-testing"
          value  = "true"
          effect = "NO_SCHEDULE"
        }
      }
    }

    playwright = {
      instance_types = ["c5.xlarge", "c5.2xlarge"]
      capacity_type  = "ON_DEMAND"
      min_size      = 0
      max_size      = 8
      desired_size  = 2
      disk_size     = 100
      disk_type     = "gp3"
      ami_type      = "AL2_x86_64"
      labels = {
        NodeGroup = "playwright"
        WorkloadType = "e2e-testing"
      }
      taints = {
        dedicated = {
          key    = "e2e-testing"
          value  = "true"
          effect = "NO_SCHEDULE"
        }
      }
    }
  }
}

# ===== DATABASE CONFIGURATION =====

variable "rds_instance_class" {
  description = "RDS instance class for PostgreSQL"
  type        = string
  default     = "db.r5.large"

  validation {
    condition = can(regex("^db\\.[a-z0-9]+\\.(nano|micro|small|medium|large|xlarge|[0-9]+xlarge)$", var.rds_instance_class))
    error_message = "RDS instance class must be valid AWS RDS instance type."
  }
}

variable "rds_allocated_storage" {
  description = "Initial allocated storage for RDS in GB"
  type        = number
  default     = 100

  validation {
    condition = var.rds_allocated_storage >= 20 && var.rds_allocated_storage <= 16384
    error_message = "RDS allocated storage must be between 20 and 16384 GB."
  }
}

variable "rds_max_allocated_storage" {
  description = "Maximum allocated storage for RDS auto-scaling in GB"
  type        = number
  default     = 500

  validation {
    condition = var.rds_max_allocated_storage >= var.rds_allocated_storage
    error_message = "RDS max allocated storage must be greater than or equal to allocated storage."
  }
}

variable "rds_backup_retention_period" {
  description = "Backup retention period in days"
  type        = number
  default     = 7

  validation {
    condition = var.rds_backup_retention_period >= 0 && var.rds_backup_retention_period <= 35
    error_message = "Backup retention period must be between 0 and 35 days."
  }
}

variable "rds_deletion_protection" {
  description = "Enable deletion protection for RDS instance"
  type        = bool
  default     = false
}

# ===== REDIS CONFIGURATION =====

variable "redis_node_type" {
  description = "ElastiCache Redis node type"
  type        = string
  default     = "cache.r6g.large"

  validation {
    condition = can(regex("^cache\\.[a-z0-9]+\\.(nano|micro|small|medium|large|xlarge|[0-9]+xlarge)$", var.redis_node_type))
    error_message = "Redis node type must be valid AWS ElastiCache node type."
  }
}

variable "redis_num_cache_clusters" {
  description = "Number of cache clusters for Redis replication group"
  type        = number
  default     = 2

  validation {
    condition = var.redis_num_cache_clusters >= 1 && var.redis_num_cache_clusters <= 6
    error_message = "Number of cache clusters must be between 1 and 6."
  }
}

variable "redis_engine_version" {
  description = "Redis engine version"
  type        = string
  default     = "7.0"

  validation {
    condition = contains(["6.2", "7.0"], var.redis_engine_version)
    error_message = "Redis engine version must be 6.2 or 7.0."
  }
}

# ===== NETWORK CONFIGURATION =====

variable "vpc_cidr" {
  description = "CIDR block for the VPC"
  type        = string
  default     = "10.0.0.0/16"

  validation {
    condition = can(cidrhost(var.vpc_cidr, 0))
    error_message = "VPC CIDR must be a valid IPv4 CIDR block."
  }
}

variable "enable_nat_gateway" {
  description = "Enable NAT Gateway for private subnets"
  type        = bool
  default     = true
}

variable "single_nat_gateway" {
  description = "Use single NAT Gateway for all private subnets"
  type        = bool
  default     = true
}

variable "enable_vpn_gateway" {
  description = "Enable VPN Gateway"
  type        = bool
  default     = false
}

# ===== SECURITY CONFIGURATION =====

variable "eks_admin_users" {
  description = "List of IAM users to add to EKS admin group"
  type = list(object({
    userarn  = string
    username = string
    groups   = list(string)
  }))
  default = []

  validation {
    condition = length(var.eks_admin_users) <= 20
    error_message = "Maximum 20 admin users can be configured."
  }
}

variable "enable_irsa" {
  description = "Enable IAM Roles for Service Accounts (IRSA)"
  type        = bool
  default     = true
}

variable "cluster_encryption_config" {
  description = "Configuration block with encryption configuration for the cluster"
  type = list(object({
    provider_key_arn = string
    resources        = list(string)
  }))
  default = []
}

# ===== MONITORING CONFIGURATION =====

variable "enable_cloudwatch_logs" {
  description = "Enable CloudWatch logs for EKS cluster"
  type        = bool
  default     = true
}

variable "cloudwatch_log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 14

  validation {
    condition = contains([1, 3, 5, 7, 14, 30, 60, 90, 120, 150, 180, 365, 400, 545, 731, 1827, 3653], var.cloudwatch_log_retention_days)
    error_message = "CloudWatch log retention must be a valid retention period."
  }
}

variable "enable_cluster_autoscaler" {
  description = "Enable cluster autoscaler"
  type        = bool
  default     = true
}

variable "enable_metrics_server" {
  description = "Enable metrics server"
  type        = bool
  default     = true
}

# ===== STORAGE CONFIGURATION =====

variable "enable_ebs_csi_driver" {
  description = "Enable Amazon EBS CSI driver"
  type        = bool
  default     = true
}

variable "enable_efs_csi_driver" {
  description = "Enable Amazon EFS CSI driver"
  type        = bool
  default     = false
}

variable "s3_bucket_force_destroy" {
  description = "Force destroy S3 buckets on terraform destroy"
  type        = bool
  default     = false
}

variable "s3_artifact_retention_days" {
  description = "Number of days to retain test artifacts in S3"
  type        = number
  default     = 30

  validation {
    condition = var.s3_artifact_retention_days >= 1 && var.s3_artifact_retention_days <= 365
    error_message = "S3 artifact retention must be between 1 and 365 days."
  }
}

# ===== HELM CHART VERSIONS =====

variable "helm_chart_versions" {
  description = "Versions for Helm charts"
  type = object({
    aws_load_balancer_controller = string
    metrics_server              = string
    cluster_autoscaler          = string
    cert_manager               = string
    external_dns               = string
    ingress_nginx              = string
  })

  default = {
    aws_load_balancer_controller = "1.6.0"
    metrics_server              = "3.11.0"
    cluster_autoscaler          = "9.29.0"
    cert_manager               = "1.13.0"
    external_dns               = "1.13.0"
    ingress_nginx              = "4.7.0"
  }
}

# ===== TESTING CONFIGURATION =====

variable "testing_config" {
  description = "Testing-specific configuration"
  type = object({
    enable_ai_testing        = bool
    enable_chaos_engineering = bool
    enable_visual_regression = bool
    max_parallel_tests      = number
    test_timeout_seconds    = number
    retry_count            = number
  })

  default = {
    enable_ai_testing        = true
    enable_chaos_engineering = true
    enable_visual_regression = true
    max_parallel_tests      = 10
    test_timeout_seconds    = 300
    retry_count            = 2
  }

  validation {
    condition = var.testing_config.max_parallel_tests >= 1 && var.testing_config.max_parallel_tests <= 50
    error_message = "Max parallel tests must be between 1 and 50."
  }

  validation {
    condition = var.testing_config.test_timeout_seconds >= 30 && var.testing_config.test_timeout_seconds <= 3600
    error_message = "Test timeout must be between 30 and 3600 seconds."
  }
}

# ===== EXTERNAL SERVICES CONFIGURATION =====

variable "openai_api_key" {
  description = "OpenAI API key for AI-powered testing features"
  type        = string
  default     = ""
  sensitive   = true
}

variable "github_token" {
  description = "GitHub token for repository access"
  type        = string
  default     = ""
  sensitive   = true
}

variable "slack_webhook_url" {
  description = "Slack webhook URL for notifications"
  type        = string
  default     = ""
  sensitive   = true
}

variable "discord_webhook_url" {
  description = "Discord webhook URL for notifications"
  type        = string
  default     = ""
  sensitive   = true
}

# ===== COST OPTIMIZATION =====

variable "enable_spot_instances" {
  description = "Enable spot instances for cost optimization"
  type        = bool
  default     = true
}

variable "spot_instance_interruption_behavior" {
  description = "Behavior when spot instance is interrupted"
  type        = string
  default     = "terminate"

  validation {
    condition = contains(["hibernate", "stop", "terminate"], var.spot_instance_interruption_behavior)
    error_message = "Spot instance interruption behavior must be hibernate, stop, or terminate."
  }
}

variable "enable_scheduled_scaling" {
  description = "Enable scheduled scaling for cost optimization"
  type        = bool
  default     = true
}

# ===== BACKUP AND DISASTER RECOVERY =====

variable "enable_cross_region_backup" {
  description = "Enable cross-region backup for disaster recovery"
  type        = bool
  default     = false
}

variable "backup_region" {
  description = "AWS region for cross-region backups"
  type        = string
  default     = "us-east-1"
}

variable "enable_point_in_time_recovery" {
  description = "Enable point-in-time recovery for RDS"
  type        = bool
  default     = true
}

# ===== COMPLIANCE AND GOVERNANCE =====

variable "enable_encryption_at_rest" {
  description = "Enable encryption at rest for all supported resources"
  type        = bool
  default     = true
}

variable "enable_encryption_in_transit" {
  description = "Enable encryption in transit for all supported resources"
  type        = bool
  default     = true
}

variable "compliance_requirements" {
  description = "List of compliance requirements to meet"
  type        = list(string)
  default     = ["soc2", "gdpr"]

  validation {
    condition = length([for req in var.compliance_requirements : req if contains(["soc2", "gdpr", "hipaa", "pci"], req)]) == length(var.compliance_requirements)
    error_message = "Compliance requirements must be from: soc2, gdpr, hipaa, pci."
  }
}

variable "enable_audit_logging" {
  description = "Enable comprehensive audit logging"
  type        = bool
  default     = true
}

# ===== TAGS =====

variable "common_tags" {
  description = "Common tags to apply to all resources"
  type        = map(string)
  default     = {}
}

variable "additional_tags" {
  description = "Additional tags for specific resource types"
  type = object({
    eks_cluster  = map(string)
    rds_instance = map(string)
    s3_bucket   = map(string)
    vpc         = map(string)
  })

  default = {
    eks_cluster  = {}
    rds_instance = {}
    s3_bucket   = {}
    vpc         = {}
  }
}

# ===== FEATURE FLAGS =====

variable "feature_flags" {
  description = "Feature flags for enabling/disabling components"
  type = object({
    enable_postgresql    = bool
    enable_clickhouse   = bool
    enable_mongodb      = bool
    enable_redis        = bool
    enable_kafka        = bool
    enable_prometheus   = bool
    enable_grafana      = bool
    enable_jaeger       = bool
    enable_elasticsearch = bool
    enable_kibana       = bool
  })

  default = {
    enable_postgresql    = true
    enable_clickhouse   = false  # Will be enabled when ClickHouse support is added
    enable_mongodb      = false  # Will be enabled when MongoDB support is added
    enable_redis        = true
    enable_kafka        = false  # Will be enabled when Kafka support is added
    enable_prometheus   = true
    enable_grafana      = true
    enable_jaeger       = true
    enable_elasticsearch = false
    enable_kibana       = false
  }
}

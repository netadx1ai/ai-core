# AI-PLATFORM Testing Infrastructure - Terraform Configuration
# Version: 1.0
# Created: 2025-01-11
# Status: ACTIVE - Implementation Phase
# Architect: architect_agent
# Classification: P0 Critical Path Foundation

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.20"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.10"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.1"
    }
  }

  backend "s3" {
    bucket         = "AI-PLATFORM-terraform-state"
    key            = "testing/terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "AI-PLATFORM-terraform-locks"
  }
}

# ===== PROVIDER CONFIGURATION =====

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "AI-PLATFORM"
      Environment = var.environment
      Component   = "testing-infrastructure"
      Owner       = "testing-team"
      ManagedBy   = "terraform"
      CostCenter  = "engineering"
      Compliance  = "soc2,gdpr"
    }
  }
}

provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

  exec {
    api_version = "client.authentication.k8s.io/v1beta1"
    command     = "aws"
    args        = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
  }
}

provider "helm" {
  kubernetes {
    host                   = module.eks.cluster_endpoint
    cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

    exec {
      api_version = "client.authentication.k8s.io/v1beta1"
      command     = "aws"
      args        = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
    }
  }
}

# ===== DATA SOURCES =====

data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

data "aws_region" "current" {}

# ===== LOCALS =====

locals {
  name            = "AI-PLATFORM-testing"
  cluster_version = "1.28"

  vpc_cidr = "10.0.0.0/16"
  azs      = slice(data.aws_availability_zones.available.names, 0, 3)

  tags = {
    Project     = "AI-PLATFORM"
    Environment = var.environment
    Component   = "testing-infrastructure"
  }
}

# ===== VPC AND NETWORKING =====

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "${local.name}-vpc"
  cidr = local.vpc_cidr

  azs             = local.azs
  private_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 4, k)]
  public_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 48)]
  intra_subnets   = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 52)]

  enable_nat_gateway = true
  single_nat_gateway = var.environment != "production"
  enable_vpn_gateway = false

  enable_dns_hostnames = true
  enable_dns_support   = true

  # VPC Flow Logs
  enable_flow_log                      = true
  create_flow_log_cloudwatch_iam_role  = true
  create_flow_log_cloudwatch_log_group = true

  # Kubernetes tags
  public_subnet_tags = {
    "kubernetes.io/role/elb" = 1
  }

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb" = 1
  }

  tags = local.tags
}

# ===== SECURITY GROUPS =====

resource "aws_security_group" "testing_cluster_sg" {
  name_prefix = "${local.name}-cluster-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    description = "TLS"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = [local.vpc_cidr]
  }

  ingress {
    description = "HTTP"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = [local.vpc_cidr]
  }

  ingress {
    description = "Testing Services"
    from_port   = 8000
    to_port     = 8200
    protocol    = "tcp"
    cidr_blocks = [local.vpc_cidr]
  }

  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  tags = merge(local.tags, {
    Name = "${local.name}-cluster-sg"
  })
}

# ===== EKS CLUSTER =====

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.15"

  cluster_name    = "${local.name}-cluster"
  cluster_version = local.cluster_version

  vpc_id                         = module.vpc.vpc_id
  subnet_ids                     = module.vpc.private_subnets
  cluster_endpoint_public_access = true

  # Encryption key
  create_kms_key = true
  cluster_encryption_config = {
    resources        = ["secrets"]
    provider_key_arn = module.eks.kms_key_arn
  }

  # EKS Managed Node Groups
  eks_managed_node_groups = {
    # General purpose nodes
    general = {
      name = "general"

      instance_types = ["t3.large"]
      capacity_type  = "ON_DEMAND"

      min_size     = 1
      max_size     = 10
      desired_size = 3

      ami_type = "AL2_x86_64"
      platform = "linux"

      subnet_ids = module.vpc.private_subnets

      # Launch template configuration
      create_launch_template = false
      launch_template_name   = ""

      disk_size = 50
      disk_type = "gp3"

      labels = {
        Environment = var.environment
        NodeGroup   = "general"
      }

      taints = {}

      tags = merge(local.tags, {
        Name = "${local.name}-general-node"
      })
    }

    # High-memory nodes for performance testing
    performance = {
      name = "performance"

      instance_types = ["r5.xlarge"]
      capacity_type  = "SPOT"

      min_size     = 0
      max_size     = 5
      desired_size = 1

      ami_type = "AL2_x86_64"
      platform = "linux"

      subnet_ids = module.vpc.private_subnets

      disk_size = 100
      disk_type = "gp3"

      labels = {
        Environment = var.environment
        NodeGroup   = "performance"
        WorkloadType = "performance-testing"
      }

      taints = {
        dedicated = {
          key    = "performance-testing"
          value  = "true"
          effect = "NO_SCHEDULE"
        }
      }

      tags = merge(local.tags, {
        Name = "${local.name}-performance-node"
      })
    }
  }

  # aws-auth configmap
  manage_aws_auth_configmap = true

  aws_auth_roles = [
    {
      rolearn  = module.eks_admins_iam_role.iam_role_arn
      username = "eks-admin"
      groups   = ["system:masters"]
    },
  ]

  aws_auth_users = var.eks_admin_users

  tags = local.tags
}

# ===== IAM ROLES =====

module "eks_admins_iam_role" {
  source  = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"
  version = "~> 5.0"

  role_name = "${local.name}-eks-admin-role"

  attach_admin_policy = true

  oidc_providers = {
    ex = {
      provider_arn               = module.eks.oidc_provider_arn
      namespace_service_accounts = ["kube-system:eks-admin"]
    }
  }

  tags = local.tags
}

# Load Balancer Controller IAM role
module "load_balancer_controller_irsa_role" {
  source  = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"
  version = "~> 5.0"

  role_name = "${local.name}-load-balancer-controller"

  attach_load_balancer_controller_policy = true

  oidc_providers = {
    ex = {
      provider_arn               = module.eks.oidc_provider_arn
      namespace_service_accounts = ["kube-system:aws-load-balancer-controller"]
    }
  }

  tags = local.tags
}

# ===== RDS INSTANCES =====

# PostgreSQL for primary test data
module "postgresql" {
  source  = "terraform-aws-modules/rds/aws"
  version = "~> 6.0"

  identifier = "${local.name}-postgresql"

  engine            = "postgres"
  engine_version    = "15.4"
  instance_class    = var.rds_instance_class
  allocated_storage = 100
  storage_encrypted = true

  db_name                = "ai_core_test"
  username               = "test_user"
  manage_master_user_password = true
  port                   = "5432"

  iam_database_authentication_enabled = true

  vpc_security_group_ids = [aws_security_group.rds.id]

  maintenance_window = "Mon:00:00-Mon:03:00"
  backup_window      = "03:00-06:00"

  # Enhanced Monitoring
  monitoring_interval    = "60"
  monitoring_role_name   = "${local.name}-rds-monitoring-role"
  create_monitoring_role = true

  # DB subnet group
  create_db_subnet_group = true
  subnet_ids            = module.vpc.private_subnets

  # DB parameter group
  family = "postgres15"

  # DB option group
  major_engine_version = "15"

  # Database Deletion Protection
  deletion_protection = var.environment == "production"

  tags = local.tags
}

# Security group for RDS
resource "aws_security_group" "rds" {
  name_prefix = "${local.name}-rds-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = [local.vpc_cidr]
  }

  tags = merge(local.tags, {
    Name = "${local.name}-rds-sg"
  })
}

# ===== ELASTICACHE REDIS =====

resource "aws_elasticache_subnet_group" "redis" {
  name       = "${local.name}-redis-subnet-group"
  subnet_ids = module.vpc.private_subnets

  tags = local.tags
}

resource "aws_security_group" "redis" {
  name_prefix = "${local.name}-redis-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port   = 6379
    to_port     = 6379
    protocol    = "tcp"
    cidr_blocks = [local.vpc_cidr]
  }

  tags = merge(local.tags, {
    Name = "${local.name}-redis-sg"
  })
}

resource "aws_elasticache_replication_group" "redis" {
  replication_group_id       = "${local.name}-redis"
  description                = "Redis cluster for AI-PLATFORM testing"

  node_type            = var.redis_node_type
  port                 = 6379
  parameter_group_name = "default.redis7"

  num_cache_clusters = 2

  engine_version             = "7.0"
  auto_minor_version_upgrade = true

  subnet_group_name  = aws_elasticache_subnet_group.redis.name
  security_group_ids = [aws_security_group.redis.id]

  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  auth_token                 = random_password.redis_auth.result

  maintenance_window = "sun:05:00-sun:09:00"

  log_delivery_configuration {
    destination      = aws_cloudwatch_log_group.redis_slow.name
    destination_type = "cloudwatch-logs"
    log_format       = "text"
    log_type         = "slow-log"
  }

  tags = local.tags
}

resource "random_password" "redis_auth" {
  length  = 32
  special = false
}

resource "aws_cloudwatch_log_group" "redis_slow" {
  name              = "/aws/elasticache/redis/${local.name}-slow-log"
  retention_in_days = 7

  tags = local.tags
}

# ===== S3 BUCKETS =====

# Bucket for test artifacts
resource "aws_s3_bucket" "test_artifacts" {
  bucket = "${local.name}-test-artifacts-${random_string.bucket_suffix.result}"

  tags = local.tags
}

resource "aws_s3_bucket_versioning" "test_artifacts" {
  bucket = aws_s3_bucket.test_artifacts.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "test_artifacts" {
  bucket = aws_s3_bucket.test_artifacts.id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "test_artifacts" {
  bucket = aws_s3_bucket.test_artifacts.id

  rule {
    id     = "delete_old_artifacts"
    status = "Enabled"

    expiration {
      days = 30
    }

    noncurrent_version_expiration {
      noncurrent_days = 7
    }
  }
}

resource "random_string" "bucket_suffix" {
  length  = 8
  special = false
  upper   = false
}

# ===== CLOUDWATCH LOG GROUPS =====

resource "aws_cloudwatch_log_group" "testing_logs" {
  name              = "/aws/testing/${local.name}"
  retention_in_days = 14

  tags = local.tags
}

# ===== SECRETS MANAGER =====

resource "aws_secretsmanager_secret" "testing_secrets" {
  name                    = "${local.name}/secrets"
  description             = "Secrets for AI-PLATFORM testing infrastructure"
  recovery_window_in_days = 7

  tags = local.tags
}

resource "aws_secretsmanager_secret_version" "testing_secrets" {
  secret_id = aws_secretsmanager_secret.testing_secrets.id
  secret_string = jsonencode({
    postgresql_password = module.postgresql.db_instance_password
    redis_auth_token   = random_password.redis_auth.result
    openai_api_key     = var.openai_api_key
    github_token       = var.github_token
    slack_webhook_url  = var.slack_webhook_url
    jwt_secret         = random_password.jwt_secret.result
  })
}

resource "random_password" "jwt_secret" {
  length  = 64
  special = true
}

# ===== HELM RELEASES =====

# AWS Load Balancer Controller
resource "helm_release" "aws_load_balancer_controller" {
  name       = "aws-load-balancer-controller"
  repository = "https://aws.github.io/eks-charts"
  chart      = "aws-load-balancer-controller"
  namespace  = "kube-system"
  version    = "1.6.0"

  set {
    name  = "clusterName"
    value = module.eks.cluster_name
  }

  set {
    name  = "serviceAccount.create"
    value = "true"
  }

  set {
    name  = "serviceAccount.name"
    value = "aws-load-balancer-controller"
  }

  set {
    name  = "serviceAccount.annotations.eks\\.amazonaws\\.com/role-arn"
    value = module.load_balancer_controller_irsa_role.iam_role_arn
  }

  depends_on = [module.eks]
}

# Metrics Server
resource "helm_release" "metrics_server" {
  name       = "metrics-server"
  repository = "https://kubernetes-sigs.github.io/metrics-server/"
  chart      = "metrics-server"
  namespace  = "kube-system"
  version    = "3.11.0"

  depends_on = [module.eks]
}

# Cluster Autoscaler
resource "helm_release" "cluster_autoscaler" {
  name       = "cluster-autoscaler"
  repository = "https://kubernetes.github.io/autoscaler"
  chart      = "cluster-autoscaler"
  namespace  = "kube-system"
  version    = "9.29.0"

  set {
    name  = "autoDiscovery.clusterName"
    value = module.eks.cluster_name
  }

  set {
    name  = "awsRegion"
    value = var.aws_region
  }

  depends_on = [module.eks]
}

# ===== OUTPUTS =====

output "cluster_endpoint" {
  description = "Endpoint for EKS control plane"
  value       = module.eks.cluster_endpoint
}

output "cluster_security_group_id" {
  description = "Security group ID attached to the EKS cluster"
  value       = module.eks.cluster_security_group_id
}

output "cluster_iam_role_name" {
  description = "IAM role name associated with EKS cluster"
  value       = module.eks.cluster_iam_role_name
}

output "cluster_certificate_authority_data" {
  description = "Base64 encoded certificate data required to communicate with the cluster"
  value       = module.eks.cluster_certificate_authority_data
}

output "cluster_name" {
  description = "The name/id of the EKS cluster"
  value       = module.eks.cluster_name
}

output "postgresql_endpoint" {
  description = "RDS PostgreSQL endpoint"
  value       = module.postgresql.db_instance_endpoint
  sensitive   = true
}

output "redis_primary_endpoint" {
  description = "Redis primary endpoint"
  value       = aws_elasticache_replication_group.redis.primary_endpoint_address
  sensitive   = true
}

output "s3_test_artifacts_bucket" {
  description = "S3 bucket for test artifacts"
  value       = aws_s3_bucket.test_artifacts.bucket
}

output "secrets_manager_arn" {
  description = "ARN of the Secrets Manager secret"
  value       = aws_secretsmanager_secret.testing_secrets.arn
}

output "vpc_id" {
  description = "ID of the VPC where resources are created"
  value       = module.vpc.vpc_id
}

output "private_subnets" {
  description = "List of IDs of private subnets"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "List of IDs of public subnets"
  value       = module.vpc.public_subnets
}

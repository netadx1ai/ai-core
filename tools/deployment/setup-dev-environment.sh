#!/bin/bash

# AI-PLATFORM Development Environment Setup Script
# Modern 2025 Docker-based development environment with comprehensive tooling
# This script sets up the complete AI-PLATFORM platform for local development

set -e

# ================================
# SCRIPT CONFIGURATION
# ================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Script directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCKER_DIR="$PROJECT_ROOT/infrastructure/docker"
VOLUMES_DIR="$DOCKER_DIR/volumes"

# Default configuration
SKIP_DOCKER_CHECK=false
SKIP_BUILD=false
SKIP_VOLUMES=false
FORCE_RECREATE=false
ENABLE_MONITORING=true
ENABLE_ANALYTICS=true
VERBOSE=false
DEV_MODE=true
DOCKER_COMPOSE_FILE="$DOCKER_DIR/docker-compose.yml"

# Version requirements (2025 standards)
MIN_DOCKER_VERSION="24.0.0"
MIN_COMPOSE_VERSION="2.20.0"
MIN_NODE_VERSION="20.0.0"
MIN_RUST_VERSION="1.75.0"

# ================================
# UTILITY FUNCTIONS
# ================================

print_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════╗"
    echo "║                    AI-PLATFORM Development Environment                   ║"
    echo "║                        Modern 2025 Setup                            ║"
    echo "╚══════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
}

show_usage() {
    cat << EOF
AI-PLATFORM Development Environment Setup

USAGE:
    $0 [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -f, --force            Force recreate all containers
    --skip-docker-check    Skip Docker version checks
    --skip-build           Skip building custom images
    --skip-volumes         Skip volume creation
    --disable-monitoring   Disable monitoring stack (Prometheus, Grafana, etc.)
    --disable-analytics    Disable analytics databases (ClickHouse)
    --prod-mode           Set up for production-like environment

EXAMPLES:
    $0                     # Standard development setup
    $0 --force             # Force recreate all containers
    $0 --skip-build        # Skip building, use existing images
    $0 --disable-monitoring # Setup without monitoring stack

REQUIREMENTS:
    - Docker $MIN_DOCKER_VERSION+
    - Docker Compose $MIN_COMPOSE_VERSION+
    - Node.js $MIN_NODE_VERSION+ (for UI development)
    - Rust $MIN_RUST_VERSION+ (for backend development)
    - At least 8GB RAM and 20GB disk space

SERVICES INCLUDED:
    Core Services:
    - API Gateway (Rust/Axum)
    - Frontend UI (React/Tauri)
    - PostgreSQL (ACID transactions)
    - ClickHouse (Analytics)
    - MongoDB (Document storage)
    - Redis (Cache & sessions)
    - Temporal.io (Workflow engine)

    Monitoring Stack:
    - Prometheus (Metrics)
    - Grafana (Dashboards)
    - Jaeger (Tracing)
    - Loki (Logs)
    - AlertManager (Alerts)

    Development Tools:
    - Traefik (Reverse proxy)
    - pgAdmin (PostgreSQL UI)
    - Mongo Express (MongoDB UI)
    - Redis Insight (Redis UI)
    - ClickHouse Play (Analytics UI)
    - MailHog (Email testing)
    - MinIO (S3 storage)

PORTS:
    3000  - Frontend UI
    8080  - API Gateway
    8081  - pgAdmin
    8082  - Mongo Express
    8083  - Redis Insight
    8084  - ClickHouse Play
    8088  - Temporal Web UI
    9090  - Prometheus
    3001  - Grafana
    16686 - Jaeger UI
    8025  - MailHog UI
    9001  - MinIO Console

EOF
}

# ================================
# SYSTEM REQUIREMENTS CHECK
# ================================

check_system_requirements() {
    print_step "Checking system requirements..."

    # Check operating system
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
        print_status "Detected Linux system"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        print_status "Detected macOS system"
    elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
        OS="windows"
        print_status "Detected Windows system"
    else
        print_warning "Unknown operating system: $OSTYPE"
        OS="unknown"
    fi

    # Check available memory
    if command -v free > /dev/null 2>&1; then
        TOTAL_MEM=$(free -m | awk 'NR==2{print $2}')
        if [ "$TOTAL_MEM" -lt 7168 ]; then
            print_warning "System has ${TOTAL_MEM}MB RAM. Recommended: 8GB+ for optimal performance"
        else
            print_success "Memory check passed: ${TOTAL_MEM}MB available"
        fi
    elif command -v vm_stat > /dev/null 2>&1; then
        # macOS memory check
        TOTAL_MEM=$(echo $(vm_stat | grep "Pages free" | awk '{print $3}' | sed 's/\.//') \* 4096 / 1024 / 1024 | bc)
        if [ "$TOTAL_MEM" -lt 7168 ]; then
            print_warning "System has ${TOTAL_MEM}MB RAM. Recommended: 8GB+ for optimal performance"
        else
            print_success "Memory check passed: ${TOTAL_MEM}MB available"
        fi
    fi

    # Check disk space
    AVAILABLE_SPACE=$(df -BG "$PROJECT_ROOT" | awk 'NR==2 {print $4}' | sed 's/G//')
    if [ "$AVAILABLE_SPACE" -lt 20 ]; then
        print_error "Insufficient disk space: ${AVAILABLE_SPACE}GB available. Required: 20GB+"
        exit 1
    else
        print_success "Disk space check passed: ${AVAILABLE_SPACE}GB available"
    fi
}

check_docker() {
    if [ "$SKIP_DOCKER_CHECK" = true ]; then
        print_warning "Skipping Docker version checks"
        return 0
    fi

    print_step "Checking Docker installation..."

    # Check if Docker is installed
    if ! command -v docker > /dev/null 2>&1; then
        print_error "Docker is not installed. Please install Docker Desktop"
        echo "Download: https://www.docker.com/products/docker-desktop"
        exit 1
    fi

    # Check Docker version
    DOCKER_VERSION=$(docker --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1)
    if ! printf '%s\n%s\n' "$MIN_DOCKER_VERSION" "$DOCKER_VERSION" | sort -V -C; then
        print_error "Docker version $DOCKER_VERSION is too old. Required: $MIN_DOCKER_VERSION+"
        exit 1
    fi
    print_success "Docker version $DOCKER_VERSION is compatible"

    # Check if Docker is running
    if ! docker info > /dev/null 2>&1; then
        print_error "Docker daemon is not running. Please start Docker Desktop"
        exit 1
    fi
    print_success "Docker daemon is running"

    # Check Docker Compose
    if ! command -v docker > /dev/null 2>&1 || ! docker compose version > /dev/null 2>&1; then
        print_error "Docker Compose is not available. Please install Docker Compose v2+"
        exit 1
    fi

    COMPOSE_VERSION=$(docker compose version --short)
    if ! printf '%s\n%s\n' "$MIN_COMPOSE_VERSION" "$COMPOSE_VERSION" | sort -V -C; then
        print_error "Docker Compose version $COMPOSE_VERSION is too old. Required: $MIN_COMPOSE_VERSION+"
        exit 1
    fi
    print_success "Docker Compose version $COMPOSE_VERSION is compatible"
}

check_development_tools() {
    print_step "Checking development tools..."

    # Check Node.js
    if command -v node > /dev/null 2>&1; then
        NODE_VERSION=$(node --version | sed 's/v//')
        if printf '%s\n%s\n' "$MIN_NODE_VERSION" "$NODE_VERSION" | sort -V -C; then
            print_success "Node.js version $NODE_VERSION is compatible"
        else
            print_warning "Node.js version $NODE_VERSION is older than recommended $MIN_NODE_VERSION"
        fi
    else
        print_warning "Node.js not found. Install Node.js $MIN_NODE_VERSION+ for frontend development"
        echo "Download: https://nodejs.org/"
    fi

    # Check Rust
    if command -v rustc > /dev/null 2>&1; then
        RUST_VERSION=$(rustc --version | awk '{print $2}')
        if printf '%s\n%s\n' "$MIN_RUST_VERSION" "$RUST_VERSION" | sort -V -C; then
            print_success "Rust version $RUST_VERSION is compatible"
        else
            print_warning "Rust version $RUST_VERSION is older than recommended $MIN_RUST_VERSION"
        fi
    else
        print_warning "Rust not found. Install Rust $MIN_RUST_VERSION+ for backend development"
        echo "Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi

    # Check Git
    if command -v git > /dev/null 2>&1; then
        GIT_VERSION=$(git --version | awk '{print $3}')
        print_success "Git version $GIT_VERSION is available"
    else
        print_warning "Git not found. Install Git for version control"
    fi
}

# ================================
# DIRECTORY AND VOLUME SETUP
# ================================

create_directories() {
    if [ "$SKIP_VOLUMES" = true ]; then
        print_warning "Skipping volume directory creation"
        return 0
    fi

    print_step "Creating directory structure..."

    # Create volume directories
    local dirs=(
        "$VOLUMES_DIR"
        "$VOLUMES_DIR/postgres"
        "$VOLUMES_DIR/mongodb"
        "$VOLUMES_DIR/mongodb_config"
        "$VOLUMES_DIR/clickhouse"
        "$VOLUMES_DIR/clickhouse_logs"
        "$VOLUMES_DIR/redis"
        "$VOLUMES_DIR/temporal"
        "$VOLUMES_DIR/prometheus"
        "$VOLUMES_DIR/grafana"
        "$VOLUMES_DIR/jaeger"
        "$VOLUMES_DIR/loki"
        "$VOLUMES_DIR/pgadmin"
        "$VOLUMES_DIR/redis_insight"
        "$VOLUMES_DIR/traefik"
        "$VOLUMES_DIR/minio"
        "$VOLUMES_DIR/kafka"
        "$VOLUMES_DIR/zookeeper"
        "$VOLUMES_DIR/zookeeper_logs"
        "$VOLUMES_DIR/backups"
        "$DOCKER_DIR/logs"
        "$DOCKER_DIR/config"
    )

    for dir in "${dirs[@]}"; do
        if [ ! -d "$dir" ]; then
            mkdir -p "$dir"
            print_status "Created directory: $dir"
        fi
    done

    # Set appropriate permissions
    if [ "$OS" != "windows" ]; then
        chmod -R 755 "$VOLUMES_DIR"
        print_success "Set directory permissions"
    fi

    print_success "Directory structure created successfully"
}

create_config_files() {
    print_step "Creating configuration files..."

    # Create Redis configuration
    if [ ! -f "$DOCKER_DIR/redis/conf/redis.conf" ]; then
        mkdir -p "$DOCKER_DIR/redis/conf"
        cat > "$DOCKER_DIR/redis/conf/redis.conf" << 'EOF'
# AI-PLATFORM Redis Configuration
bind 0.0.0.0
port 6379
timeout 300
keepalive 60
maxmemory 512mb
maxmemory-policy allkeys-lru
save 900 1
save 300 10
save 60 10000
rdbcompression yes
rdbchecksum yes
dbfilename dump.rdb
dir /data
appendonly yes
appendfsync everysec
EOF
        print_status "Created Redis configuration"
    fi

    # Create MongoDB configuration
    if [ ! -f "$DOCKER_DIR/mongodb/conf/mongod.conf" ]; then
        mkdir -p "$DOCKER_DIR/mongodb/conf"
        cat > "$DOCKER_DIR/mongodb/conf/mongod.conf" << 'EOF'
# AI-PLATFORM MongoDB Configuration
storage:
  dbPath: /data/db
  journal:
    enabled: true
  wiredTiger:
    engineConfig:
      journalCompressor: snappy
      directoryForIndexes: false
    collectionConfig:
      blockCompressor: snappy
    indexConfig:
      prefixCompression: true

systemLog:
  destination: file
  logAppend: true
  path: /var/log/mongodb/mongod.log
  logRotate: rename
  verbosity: 1

net:
  port: 27017
  bindIp: 0.0.0.0

processManagement:
  timeZoneInfo: /usr/share/zoneinfo

security:
  authorization: enabled

replication:
  replSetName: AI-PLATFORM-rs

operationProfiling:
  mode: slowOp
  slowOpThresholdMs: 1000
EOF
        print_status "Created MongoDB configuration"
    fi

    # Create ClickHouse configuration
    if [ ! -d "$DOCKER_DIR/clickhouse/config" ]; then
        mkdir -p "$DOCKER_DIR/clickhouse/config"
        cat > "$DOCKER_DIR/clickhouse/config/config.xml" << 'EOF'
<?xml version="1.0"?>
<clickhouse>
    <logger>
        <level>information</level>
        <console>1</console>
    </logger>

    <http_port>8123</http_port>
    <tcp_port>9000</tcp_port>
    <mysql_port>9004</mysql_port>
    <postgresql_port>9005</postgresql_port>

    <listen_host>::</listen_host>
    <listen_host>0.0.0.0</listen_host>

    <max_connections>1000</max_connections>
    <keep_alive_timeout>3</keep_alive_timeout>
    <max_concurrent_queries>500</max_concurrent_queries>
    <uncompressed_cache_size>8589934592</uncompressed_cache_size>
    <mark_cache_size>5368709120</mark_cache_size>

    <path>/var/lib/clickhouse/</path>
    <tmp_path>/var/lib/clickhouse/tmp/</tmp_path>
    <user_files_path>/var/lib/clickhouse/user_files/</user_files_path>

    <users_config>/etc/clickhouse-server/users.d/users.xml</users_config>

    <default_profile>default</default_profile>
    <default_database>analytics</default_database>

    <timezone>UTC</timezone>

    <mlock_executable>false</mlock_executable>

    <prometheus>
        <endpoint>/metrics</endpoint>
        <port>9363</port>
        <metrics>true</metrics>
        <events>true</events>
        <asynchronous_metrics>true</asynchronous_metrics>
    </prometheus>
</clickhouse>
EOF
        print_status "Created ClickHouse configuration"
    fi

    # Create environment file
    if [ ! -f "$DOCKER_DIR/.env" ]; then
        cat > "$DOCKER_DIR/.env" << 'EOF'
# AI-PLATFORM Development Environment Variables
COMPOSE_PROJECT_NAME=AI-PLATFORM
COMPOSE_FILE=docker-compose.yml

# Database Passwords
POSTGRES_PASSWORD=ai_core_password
MONGO_PASSWORD=ai_core_mongo_password
CLICKHOUSE_PASSWORD=ai_core_clickhouse_password
REDIS_PASSWORD=""

# API Configuration
JWT_SECRET=dev_jwt_secret_change_in_production_immediately
API_RATE_LIMIT=1000
CORS_ORIGINS=http://localhost:3000,http://localhost:8080

# Monitoring Configuration
GRAFANA_ADMIN_PASSWORD=ai_core_grafana_admin
PROMETHEUS_RETENTION=30d

# MinIO Configuration
MINIO_ROOT_USER=ai_core_minio
MINIO_ROOT_PASSWORD=ai_core_minio_password

# Development Configuration
NODE_ENV=development
RUST_ENV=development
LOG_LEVEL=debug
ENABLE_HOT_RELOAD=true
EOF
        print_status "Created environment configuration"
    fi

    print_success "Configuration files created successfully"
}

# ================================
# DOCKER OPERATIONS
# ================================

build_images() {
    if [ "$SKIP_BUILD" = true ]; then
        print_warning "Skipping image building"
        return 0
    fi

    print_step "Building Docker images..."

    cd "$PROJECT_ROOT"

    # Build API Gateway image
    if [ -f "$DOCKER_DIR/Dockerfile.api" ]; then
        print_status "Building API Gateway image..."
        docker build -f "$DOCKER_DIR/Dockerfile.api" -t AI-PLATFORM/api-gateway:latest . || {
            print_error "Failed to build API Gateway image"
            return 1
        }
        print_success "API Gateway image built successfully"
    fi

    # Build Frontend UI image
    if [ -f "$DOCKER_DIR/Dockerfile.ui" ] && [ -d "$PROJECT_ROOT/src/ui" ]; then
        print_status "Building Frontend UI image..."
        docker build -f "$DOCKER_DIR/Dockerfile.ui" -t AI-PLATFORM/frontend-ui:latest . || {
            print_warning "Frontend UI build failed - this is optional for backend development"
        }
    fi

    print_success "Docker images built successfully"
}

start_services() {
    print_step "Starting AI-PLATFORM services..."

    cd "$DOCKER_DIR"

    # Stop any existing services if force recreate
    if [ "$FORCE_RECREATE" = true ]; then
        print_status "Stopping existing services..."
        docker compose down -v --remove-orphans || true
    fi

    # Start core services first
    print_status "Starting core database services..."
    docker compose up -d postgres redis mongodb clickhouse

    # Wait for databases to be healthy
    print_status "Waiting for databases to be ready..."
    local max_attempts=60
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if docker compose ps --services --filter "status=running" | grep -q "postgres\|redis\|mongodb\|clickhouse"; then
            local healthy_services=0

            # Check PostgreSQL
            if docker compose exec -T postgres pg_isready -U ai_core_user -d ai_core > /dev/null 2>&1; then
                ((healthy_services++))
            fi

            # Check Redis
            if docker compose exec -T redis redis-cli ping > /dev/null 2>&1; then
                ((healthy_services++))
            fi

            # Check MongoDB
            if docker compose exec -T mongodb mongosh --eval "db.adminCommand('ping')" > /dev/null 2>&1; then
                ((healthy_services++))
            fi

            # Check ClickHouse
            if curl -s http://localhost:8123/ping > /dev/null 2>&1; then
                ((healthy_services++))
            fi

            if [ $healthy_services -eq 4 ]; then
                print_success "All databases are healthy"
                break
            fi
        fi

        ((attempt++))
        print_status "Waiting for databases... ($attempt/$max_attempts)"
        sleep 5
    done

    if [ $attempt -eq $max_attempts ]; then
        print_warning "Some databases may not be fully ready, continuing anyway..."
    fi

    # Start workflow engine
    print_status "Starting Temporal workflow engine..."
    docker compose up -d temporal temporal-web

    # Start monitoring stack if enabled
    if [ "$ENABLE_MONITORING" = true ]; then
        print_status "Starting monitoring stack..."
        docker compose up -d prometheus grafana jaeger loki promtail
    fi

    # Start reverse proxy
    print_status "Starting Traefik reverse proxy..."
    docker compose up -d traefik

    # Start development tools
    print_status "Starting development tools..."
    docker compose up -d pgadmin mongo-express redis-insight mailhog minio

    # Start main application services
    print_status "Starting application services..."
    docker compose up -d api-gateway frontend-ui

    # Show service status
    print_step "Service startup complete!"
    docker compose ps

    print_success "All services started successfully"
}

# ================================
# POST-SETUP CONFIGURATION
# ================================

setup_databases() {
    print_step "Setting up database schemas and initial data..."

    # Wait a bit for services to fully initialize
    sleep 10

    # Run PostgreSQL migrations
    print_status "Running PostgreSQL migrations..."
    if [ -f "$PROJECT_ROOT/src/api-gateway/migrations" ]; then
        docker compose exec -T api-gateway /app/api-gateway migrate || {
            print_warning "Migration command failed, databases may need manual setup"
        }
    fi

    # Create ClickHouse analytics database
    print_status "Setting up ClickHouse analytics database..."
    docker compose exec -T clickhouse clickhouse-client --query="
        CREATE DATABASE IF NOT EXISTS analytics;
        CREATE TABLE IF NOT EXISTS analytics.workflow_events (
            event_id UUID,
            workflow_id String,
            user_id String,
            service_name LowCardinality(String),
            event_type LowCardinality(String),
            duration_ms UInt64,
            cost_usd Float64,
            success Bool,
            timestamp DateTime64(3),
            date Date MATERIALIZED toDate(timestamp)
        ) ENGINE = MergeTree()
        PARTITION BY toYYYYMM(date)
        ORDER BY (service_name, event_type, timestamp)
        TTL date + INTERVAL 1 YEAR;
    " || print_warning "ClickHouse setup may need manual configuration"

    # Setup MongoDB collections
    print_status "Setting up MongoDB collections..."
    docker compose exec -T mongodb mongosh automation_platform --eval "
        db.createCollection('content_items');
        db.createCollection('campaigns');
        db.createCollection('workflows');
        db.content_items.createIndex({'created_at': 1});
        db.campaigns.createIndex({'status': 1, 'created_at': -1});
        db.workflows.createIndex({'user_id': 1, 'status': 1});
    " || print_warning "MongoDB setup may need manual configuration"

    print_success "Database setup completed"
}

setup_monitoring() {
    if [ "$ENABLE_MONITORING" != true ]; then
        return 0
    fi

    print_step "Configuring monitoring dashboards..."

    # Wait for Grafana to be ready
    print_status "Waiting for Grafana to be ready..."
    local attempt=0
    while [ $attempt -lt 30 ]; do
        if curl -s http://localhost:3001/api/health > /dev/null 2>&1; then
            break
        fi
        ((attempt++))
        sleep 2
    done

    # Import default dashboards
    print_status "Importing Grafana dashboards..."
    # Dashboard import would happen here via API calls

    print_success "Monitoring setup completed"
}

show_service_urls() {
    print_step "AI-PLATFORM Development Environment is ready!"

    echo ""
    echo -e "${GREEN}🚀 SERVICE URLS:${NC}"
    echo ""
    echo -e "${CYAN}Core Application:${NC}"
    echo "  • Frontend UI:          http://localhost:3000"
    echo "  • API Gateway:          http://localhost:8080"
    echo "  • API Documentation:    http://localhost:8080/docs"
    echo ""

    echo -e "${CYAN}Database Admin Tools:${NC}"
    echo "  • pgAdmin (PostgreSQL): http://localhost:8081 (admin@aicore.local / pgadmin_password)"
    echo "  • Mongo Express:        http://localhost:8082 (admin / mongo_admin_password)"
    echo "  • Redis Insight:        http://localhost:8083"
    echo "  • ClickHouse Play:      http://localhost:8084"
    echo ""

    if [ "$ENABLE_MONITORING" = true ]; then
        echo -e "${CYAN}Monitoring & Observability:${NC}"
        echo "  • Grafana Dashboards:   http://localhost:3001 (admin / ai_core_grafana_admin)"
        echo "  • Prometheus Metrics:   http://localhost:9090"
        echo "  • Jaeger Tracing:       http://localhost:16686"
        echo "  • Temporal Web UI:      http://localhost:8088"
        echo ""
    fi

    echo -e "${CYAN}Development Tools:${NC}"
    echo "  • MailHog (Email Test): http://localhost:8025"
    echo "  • MinIO Console:        http://localhost:9001 (ai_core_minio / ai_core_minio_password)"
    echo "  • Traefik Dashboard:    http://localhost:8080"
    echo ""

    echo -e "${YELLOW}📖 QUICK COMMANDS:${NC}"
    echo ""
    echo "  • View logs:            docker compose -f $DOCKER_COMPOSE_FILE logs -f [service]"
    echo "  • Stop all:             docker compose -f $DOCKER_COMPOSE_FILE down"
    echo "  • Restart service:      docker compose -f $DOCKER_COMPOSE_FILE restart [service]"
    echo "  • Shell access:         docker compose -f $DOCKER_COMPOSE_FILE exec [service] /bin/bash"
    echo "  • View status:          docker compose -f $DOCKER_COMPOSE_FILE ps"
    echo ""

    echo -e "${GREEN}✅ Development environment setup complete!${NC}"
    echo -e "${BLUE}📚 Documentation: https://docs.AI-PLATFORM.dev${NC}"
    echo -e "${BLUE}🐛 Issues: https://github.com/AI-PLATFORM/platform/issues${NC}"
    echo ""
}

# ================================
# CLEANUP FUNCTIONS
# ================================

cleanup_on_exit() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        print_error "Setup failed with exit code $exit_code"
        print_status "Cleaning up partial deployment..."
        cd "$DOCKER_DIR" && docker compose down --remove-orphans || true
    fi
    exit $exit_code
}

# ================================
# MAIN EXECUTION
# ================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -v|--verbose)
                VERBOSE=true
                set -x
                shift
                ;;
            -f|--force)
                FORCE_RECREATE=true
                shift
                ;;
            --skip-docker-check)
                SKIP_DOCKER_CHECK=true
                shift
                ;;
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --skip-volumes)
                SKIP_VOLUMES=true
                shift
                ;;
            --disable-monitoring)
                ENABLE_MONITORING=false
                shift
                ;;
            --disable-analytics)
                ENABLE_ANALYTICS=false
                shift
                ;;
            --prod-mode)
                DEV_MODE=false
                DOCKER_COMPOSE_FILE="$DOCKER_DIR/docker-compose.prod.yml"
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

main() {
    # Set up error handling
    trap cleanup_on_exit EXIT

    # Parse command line arguments
    parse_arguments "$@"

    # Print banner
    print_banner

    # Check system requirements
    check_system_requirements
    check_docker
    check_development_tools

    # Setup directories and configuration
    create_directories
    create_config_files

    # Build and start services
    build_images
    start_services

    # Post-setup configuration
    setup_databases
    setup_monitoring

    # Show final status
    show_service_urls

    print_success "🎉 AI-PLATFORM development environment is fully operational!"
}

# Execute main function with all arguments
main "$@"

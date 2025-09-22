#!/bin/bash

# AI-PLATFORM Test Environment Management Script
# FAANG-Enhanced Testing Infrastructure - DevOps Agent Implementation T7.1
#
# Comprehensive test environment management with:
# - Environment setup and teardown
# - Service health monitoring
# - Test execution coordination
# - Log aggregation and analysis
# - Performance monitoring
# - Cleanup and maintenance

set -euo pipefail

# ============================================================================
# Configuration and Constants
# ============================================================================

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly COMPOSE_FILE="${PROJECT_ROOT}/infrastructure/test-environments/docker-compose.test.yml"
readonly LOG_DIR="${PROJECT_ROOT}/logs"
readonly TEST_RESULTS_DIR="${PROJECT_ROOT}/test-results"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly PURPLE='\033[0;35m'
readonly CYAN='\033[0;36m'
readonly WHITE='\033[1;37m'
readonly NC='\033[0m' # No Color

# Default configuration
readonly DEFAULT_ENV="testing"
readonly DEFAULT_PROFILE="core"
readonly HEALTH_CHECK_TIMEOUT=300
readonly STARTUP_TIMEOUT=120

# ============================================================================
# Utility Functions
# ============================================================================

log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    case "$level" in
        "INFO")
            echo -e "${timestamp} ${BLUE}[INFO]${NC} $message"
            ;;
        "WARN")
            echo -e "${timestamp} ${YELLOW}[WARN]${NC} $message"
            ;;
        "ERROR")
            echo -e "${timestamp} ${RED}[ERROR]${NC} $message" >&2
            ;;
        "SUCCESS")
            echo -e "${timestamp} ${GREEN}[SUCCESS]${NC} $message"
            ;;
        "DEBUG")
            if [[ "${DEBUG:-0}" == "1" ]]; then
                echo -e "${timestamp} ${PURPLE}[DEBUG]${NC} $message"
            fi
            ;;
    esac
}

spinner() {
    local pid=$1
    local message="$2"
    local spin='-\|/'
    local i=0

    while kill -0 $pid 2>/dev/null; do
        i=$(( (i+1) %4 ))
        printf "\r${CYAN}[${spin:$i:1}]${NC} $message..."
        sleep .1
    done
    printf "\r"
}

check_dependencies() {
    log "INFO" "Checking dependencies..."

    local deps=("docker" "docker-compose" "curl" "jq")
    local missing=()

    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing+=("$dep")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        log "ERROR" "Missing dependencies: ${missing[*]}"
        log "ERROR" "Please install the missing dependencies and try again"
        exit 1
    fi

    # Check Docker is running
    if ! docker info &> /dev/null; then
        log "ERROR" "Docker is not running. Please start Docker and try again"
        exit 1
    fi

    log "SUCCESS" "All dependencies are available"
}

create_directories() {
    log "INFO" "Creating necessary directories..."

    local dirs=(
        "$LOG_DIR"
        "$TEST_RESULTS_DIR"
        "${TEST_RESULTS_DIR}/e2e"
        "${TEST_RESULTS_DIR}/performance"
        "${TEST_RESULTS_DIR}/integration"
        "${TEST_RESULTS_DIR}/security"
        "${PROJECT_ROOT}/infrastructure/test-environments/logs"
        "${PROJECT_ROOT}/infrastructure/test-environments/test-results"
        "${PROJECT_ROOT}/infrastructure/test-environments/minio-data"
    )

    for dir in "${dirs[@]}"; do
        mkdir -p "$dir"
        log "DEBUG" "Created directory: $dir"
    done

    log "SUCCESS" "Directories created successfully"
}

# ============================================================================
# Environment Management Functions
# ============================================================================

start_environment() {
    local profile="${1:-$DEFAULT_PROFILE}"

    log "INFO" "Starting test environment with profile: $profile"

    create_directories

    # Start services based on profile
    case "$profile" in
        "core")
            docker-compose -f "$COMPOSE_FILE" up -d postgres-test clickhouse-test mongodb-test redis-test
            ;;
        "services")
            docker-compose -f "$COMPOSE_FILE" up -d postgres-test clickhouse-test mongodb-test redis-test test-data-api auth-service-test
            ;;
        "full")
            docker-compose -f "$COMPOSE_FILE" up -d
            ;;
        "monitoring")
            docker-compose -f "$COMPOSE_FILE" up -d postgres-test clickhouse-test mongodb-test redis-test prometheus-test grafana-test
            ;;
        *)
            log "ERROR" "Unknown profile: $profile"
            log "INFO" "Available profiles: core, services, full, monitoring"
            exit 1
            ;;
    esac

    log "INFO" "Waiting for services to start..."
    wait_for_services "$profile"

    log "SUCCESS" "Test environment started successfully"
    show_service_urls
}

stop_environment() {
    log "INFO" "Stopping test environment..."

    docker-compose -f "$COMPOSE_FILE" down --remove-orphans

    log "SUCCESS" "Test environment stopped"
}

restart_environment() {
    local profile="${1:-$DEFAULT_PROFILE}"

    log "INFO" "Restarting test environment..."

    stop_environment
    sleep 2
    start_environment "$profile"
}

cleanup_environment() {
    local full_cleanup="${1:-false}"

    log "INFO" "Cleaning up test environment..."

    # Stop and remove containers
    docker-compose -f "$COMPOSE_FILE" down --remove-orphans

    if [[ "$full_cleanup" == "true" ]]; then
        log "WARN" "Performing full cleanup (removing volumes and data)..."
        docker-compose -f "$COMPOSE_FILE" down -v

        # Remove test data directories
        rm -rf "${PROJECT_ROOT}/infrastructure/test-environments/minio-data"
        rm -rf "${PROJECT_ROOT}/infrastructure/test-environments/logs"

        # Prune unused Docker resources
        docker system prune -f
        docker volume prune -f
    fi

    log "SUCCESS" "Cleanup completed"
}

# ============================================================================
# Service Health Monitoring
# ============================================================================

check_service_health() {
    local service="$1"
    local url="$2"
    local timeout="${3:-30}"

    local count=0

    while [[ $count -lt $timeout ]]; do
        if curl -s -f "$url" > /dev/null 2>&1; then
            return 0
        fi
        sleep 1
        ((count++))
    done

    return 1
}

wait_for_services() {
    local profile="$1"

    log "INFO" "Waiting for services to be healthy..."

    local services_to_check=()

    case "$profile" in
        "core")
            services_to_check=(
                "PostgreSQL:http://localhost:5432"
                "ClickHouse:http://localhost:8123/ping"
                "MongoDB:http://localhost:27017"
                "Redis:http://localhost:6379"
            )
            ;;
        "services"|"full")
            services_to_check=(
                "PostgreSQL:http://localhost:5432"
                "ClickHouse:http://localhost:8123/ping"
                "MongoDB:http://localhost:27017"
                "Redis:http://localhost:6379"
                "Test Data API:http://localhost:8002/health"
                "Auth Service:http://localhost:8001/health"
            )
            ;;
        "monitoring")
            services_to_check=(
                "PostgreSQL:http://localhost:5432"
                "ClickHouse:http://localhost:8123/ping"
                "MongoDB:http://localhost:27017"
                "Redis:http://localhost:6379"
                "Prometheus:http://localhost:9090/-/healthy"
                "Grafana:http://localhost:3001/api/health"
            )
            ;;
    esac

    local failed_services=()

    for service_info in "${services_to_check[@]}"; do
        local service_name="${service_info%%:*}"
        local service_url="${service_info#*:}"

        log "INFO" "Checking $service_name..."

        if [[ "$service_name" == "PostgreSQL" ]]; then
            # Special handling for PostgreSQL
            if ! docker-compose -f "$COMPOSE_FILE" exec -T postgres-test pg_isready -U postgres > /dev/null 2>&1; then
                failed_services+=("$service_name")
            fi
        elif [[ "$service_name" == "MongoDB" ]]; then
            # Special handling for MongoDB
            if ! docker-compose -f "$COMPOSE_FILE" exec -T mongodb-test mongosh --eval "db.adminCommand('ping')" > /dev/null 2>&1; then
                failed_services+=("$service_name")
            fi
        elif [[ "$service_name" == "Redis" ]]; then
            # Special handling for Redis
            if ! docker-compose -f "$COMPOSE_FILE" exec -T redis-test redis-cli -a test_redis_123 ping > /dev/null 2>&1; then
                failed_services+=("$service_name")
            fi
        else
            # HTTP health check
            if ! check_service_health "$service_name" "$service_url" 60; then
                failed_services+=("$service_name")
            fi
        fi

        if [[ " ${failed_services[@]} " =~ " ${service_name} " ]]; then
            log "ERROR" "$service_name is not healthy"
        else
            log "SUCCESS" "$service_name is healthy"
        fi
    done

    if [[ ${#failed_services[@]} -gt 0 ]]; then
        log "ERROR" "Failed services: ${failed_services[*]}"
        return 1
    fi

    log "SUCCESS" "All services are healthy"
    return 0
}

show_service_status() {
    log "INFO" "Service Status:"
    echo

    # Get running containers
    local containers=$(docker-compose -f "$COMPOSE_FILE" ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}")

    if [[ -n "$containers" ]]; then
        echo "$containers"
    else
        log "WARN" "No containers are running"
    fi

    echo
}

show_service_urls() {
    log "INFO" "Service URLs:"
    echo
    printf "%-20s %s\n" "Service" "URL"
    printf "%-20s %s\n" "-------" "---"
    printf "%-20s %s\n" "PostgreSQL" "postgresql://postgres:test_password_secure_123@localhost:5432/aicore_test"
    printf "%-20s %s\n" "ClickHouse" "http://localhost:8123"
    printf "%-20s %s\n" "MongoDB" "mongodb://admin:test_mongo_123@localhost:27017/aicore_test"
    printf "%-20s %s\n" "Redis" "redis://:test_redis_123@localhost:6379/0"
    printf "%-20s %s\n" "Test Data API" "http://localhost:8002"
    printf "%-20s %s\n" "Auth Service" "http://localhost:8001"
    printf "%-20s %s\n" "Frontend" "http://localhost:3000"
    printf "%-20s %s\n" "Prometheus" "http://localhost:9090"
    printf "%-20s %s\n" "Grafana" "http://localhost:3001 (admin/test_grafana_123)"
    printf "%-20s %s\n" "Jaeger" "http://localhost:16686"
    printf "%-20s %s\n" "MailHog" "http://localhost:8025"
    printf "%-20s %s\n" "MinIO" "http://localhost:9002 (testuser/testpass123)"
    echo
}

# ============================================================================
# Test Execution Functions
# ============================================================================

run_tests() {
    local test_type="$1"
    local additional_args="${2:-}"

    log "INFO" "Running $test_type tests..."

    case "$test_type" in
        "unit")
            run_unit_tests "$additional_args"
            ;;
        "integration")
            run_integration_tests "$additional_args"
            ;;
        "e2e")
            run_e2e_tests "$additional_args"
            ;;
        "performance")
            run_performance_tests "$additional_args"
            ;;
        "all")
            run_unit_tests "$additional_args"
            run_integration_tests "$additional_args"
            run_e2e_tests "$additional_args"
            ;;
        *)
            log "ERROR" "Unknown test type: $test_type"
            log "INFO" "Available types: unit, integration, e2e, performance, all"
            exit 1
            ;;
    esac
}

run_unit_tests() {
    local args="$1"

    log "INFO" "Running unit tests..."

    cd "$PROJECT_ROOT"

    # Run Rust unit tests
    if [[ -d "src" ]]; then
        for component in src/*/; do
            if [[ -f "${component}Cargo.toml" ]]; then
                local component_name=$(basename "$component")
                log "INFO" "Running unit tests for $component_name..."

                cd "$component"
                cargo test --verbose $args
                cd "$PROJECT_ROOT"
            fi
        done
    fi

    log "SUCCESS" "Unit tests completed"
}

run_integration_tests() {
    local args="$1"

    log "INFO" "Running integration tests..."

    # Ensure test environment is running
    if ! wait_for_services "services"; then
        log "ERROR" "Test services are not healthy. Cannot run integration tests."
        exit 1
    fi

    cd "$PROJECT_ROOT"

    # Run integration tests
    cargo test --test "*integration*" --verbose $args

    log "SUCCESS" "Integration tests completed"
}

run_e2e_tests() {
    local args="$1"

    log "INFO" "Running E2E tests..."

    # Start E2E test container
    docker-compose -f "$COMPOSE_FILE" --profile e2e-testing run --rm playwright-test npx playwright test $args

    log "SUCCESS" "E2E tests completed"
}

run_performance_tests() {
    local test_type="${1:-load}"

    log "INFO" "Running performance tests: $test_type"

    # Set environment variables for K6
    export TEST_TYPE="$test_type"
    export BASE_URL="http://localhost:8000"

    # Run K6 performance tests
    docker-compose -f "$COMPOSE_FILE" --profile performance-testing run --rm k6-test run /scripts/performance-test.js

    log "SUCCESS" "Performance tests completed"
}

# ============================================================================
# Data Management Functions
# ============================================================================

seed_test_data() {
    local data_type="${1:-all}"

    log "INFO" "Seeding test data: $data_type"

    # Run data seeder
    docker-compose -f "$COMPOSE_FILE" --profile data-seeding run --rm data-seeder

    log "SUCCESS" "Test data seeded successfully"
}

backup_test_data() {
    local backup_name="${1:-$(date +%Y%m%d_%H%M%S)}"
    local backup_dir="${PROJECT_ROOT}/backups"

    log "INFO" "Creating test data backup: $backup_name"

    mkdir -p "$backup_dir"

    # Backup PostgreSQL
    docker-compose -f "$COMPOSE_FILE" exec -T postgres-test pg_dump -U postgres aicore_test > "${backup_dir}/postgres_${backup_name}.sql"

    # Backup MongoDB
    docker-compose -f "$COMPOSE_FILE" exec -T mongodb-test mongodump --db aicore_test --archive > "${backup_dir}/mongodb_${backup_name}.archive"

    # Backup Redis (if needed)
    docker-compose -f "$COMPOSE_FILE" exec -T redis-test redis-cli -a test_redis_123 --rdb - > "${backup_dir}/redis_${backup_name}.rdb"

    log "SUCCESS" "Backup created: $backup_name"
}

restore_test_data() {
    local backup_name="$1"
    local backup_dir="${PROJECT_ROOT}/backups"

    if [[ -z "$backup_name" ]]; then
        log "ERROR" "Backup name is required"
        exit 1
    fi

    log "INFO" "Restoring test data from backup: $backup_name"

    # Restore PostgreSQL
    if [[ -f "${backup_dir}/postgres_${backup_name}.sql" ]]; then
        docker-compose -f "$COMPOSE_FILE" exec -T postgres-test psql -U postgres -d aicore_test < "${backup_dir}/postgres_${backup_name}.sql"
    fi

    # Restore MongoDB
    if [[ -f "${backup_dir}/mongodb_${backup_name}.archive" ]]; then
        docker-compose -f "$COMPOSE_FILE" exec -T mongodb-test mongorestore --db aicore_test --archive < "${backup_dir}/mongodb_${backup_name}.archive"
    fi

    log "SUCCESS" "Data restored from backup: $backup_name"
}

# ============================================================================
# Monitoring and Logging Functions
# ============================================================================

show_logs() {
    local service="${1:-all}"
    local follow="${2:-false}"

    if [[ "$service" == "all" ]]; then
        if [[ "$follow" == "true" ]]; then
            docker-compose -f "$COMPOSE_FILE" logs -f
        else
            docker-compose -f "$COMPOSE_FILE" logs --tail=100
        fi
    else
        if [[ "$follow" == "true" ]]; then
            docker-compose -f "$COMPOSE_FILE" logs -f "$service"
        else
            docker-compose -f "$COMPOSE_FILE" logs --tail=100 "$service"
        fi
    fi
}

collect_metrics() {
    log "INFO" "Collecting system metrics..."

    local metrics_file="${TEST_RESULTS_DIR}/system_metrics_$(date +%Y%m%d_%H%M%S).json"

    # Collect Docker stats
    docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}" > "${metrics_file%.json}.txt"

    # Collect service health metrics
    {
        echo "{"
        echo "  \"timestamp\": \"$(date -Iseconds)\","
        echo "  \"services\": {"

        local services=(
            "postgres-test:5432"
            "clickhouse-test:8123"
            "mongodb-test:27017"
            "redis-test:6379"
            "test-data-api:8002"
            "auth-service-test:8001"
        )

        local first=true
        for service_info in "${services[@]}"; do
            local service="${service_info%%:*}"
            local port="${service_info#*:}"

            if [[ "$first" == "true" ]]; then
                first=false
            else
                echo ","
            fi

            local status="down"
            if docker-compose -f "$COMPOSE_FILE" ps "$service" | grep -q "Up"; then
                status="up"
            fi

            echo -n "    \"$service\": { \"status\": \"$status\", \"port\": $port }"
        done

        echo ""
        echo "  }"
        echo "}"
    } > "$metrics_file"

    log "SUCCESS" "Metrics collected: $metrics_file"
}

generate_report() {
    log "INFO" "Generating test environment report..."

    local report_file="${TEST_RESULTS_DIR}/environment_report_$(date +%Y%m%d_%H%M%S).html"

    cat > "$report_file" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>AI-PLATFORM Test Environment Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 8px; }
        .section { margin: 20px 0; }
        table { width: 100%; border-collapse: collapse; }
        th, td { padding: 10px; border: 1px solid #ddd; text-align: left; }
        th { background: #f8f9fa; }
        .status-up { color: #28a745; }
        .status-down { color: #dc3545; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🧪 AI-PLATFORM Test Environment Report</h1>
        <p><strong>Generated:</strong> $(date)</p>
        <p><strong>Environment:</strong> $DEFAULT_ENV</p>
    </div>

    <div class="section">
        <h2>Service Status</h2>
        <table>
            <tr><th>Service</th><th>Status</th><th>Port</th><th>URL</th></tr>
EOF

    # Add service status to report
    local services=(
        "PostgreSQL:5432:postgresql://localhost:5432/aicore_test"
        "ClickHouse:8123:http://localhost:8123"
        "MongoDB:27017:mongodb://localhost:27017/aicore_test"
        "Redis:6379:redis://localhost:6379"
        "Test Data API:8002:http://localhost:8002"
        "Auth Service:8001:http://localhost:8001"
        "Prometheus:9090:http://localhost:9090"
        "Grafana:3001:http://localhost:3001"
    )

    for service_info in "${services[@]}"; do
        local service_name="${service_info%%:*}"
        local service_port="${service_info#*:}"
        service_port="${service_port%%:*}"
        local service_url="${service_info##*:}"

        local status_class="status-down"
        local status_text="Down"

        if curl -s -f "http://localhost:$service_port" > /dev/null 2>&1 || \
           curl -s -f "http://localhost:$service_port/health" > /dev/null 2>&1 || \
           curl -s -f "http://localhost:$service_port/ping" > /dev/null 2>&1; then
            status_class="status-up"
            status_text="Up"
        fi

        cat >> "$report_file" << EOF
            <tr>
                <td>$service_name</td>
                <td class="$status_class">$status_text</td>
                <td>$service_port</td>
                <td><a href="$service_url">$service_url</a></td>
            </tr>
EOF
    done

    cat >> "$report_file" << 'EOF'
        </table>
    </div>

    <div class="section">
        <h2>System Information</h2>
        <p>Docker Version: $(docker --version)</p>
        <p>Docker Compose Version: $(docker-compose --version)</p>
        <p>Host OS: $(uname -a)</p>
    </div>
</body>
</html>
EOF

    log "SUCCESS" "Report generated: $report_file"
}

# ============================================================================
# Main Function and Command Handling
# ============================================================================

show_help() {
    cat << 'EOF'
AI-PLATFORM Test Environment Management Script

USAGE:
    ./test-environment.sh <command> [options]

COMMANDS:
    start [profile]           Start test environment (profiles: core, services, full, monitoring)
    stop                      Stop test environment
    restart [profile]         Restart test environment
    cleanup [full]            Clean up environment (use 'full' for complete cleanup)

    status                    Show service status
    health                    Check service health
    logs [service] [follow]   Show logs (use 'follow' to tail logs)
    urls                      Show service URLs

    test <type> [args]        Run tests (types: unit, integration, e2e, performance, all)
    e2e [args]                Run E2E tests with Playwright
    perf [type]               Run performance tests (types: load, stress, spike, endurance)

    seed [type]               Seed test data
    backup [name]             Backup test data
    restore <name>            Restore test data from backup

    metrics                   Collect system metrics
    report                    Generate environment report

    help                      Show this help message

EXAMPLES:
    # Start core services only
    ./test-environment.sh start core

    # Start all services with monitoring
    ./test-environment.sh start full

    # Run E2E tests
    ./test-environment.sh test e2e

    # Run performance tests
    ./test-environment.sh perf load

    # Follow logs for specific service
    ./test-environment.sh logs test-data-api follow

    # Full cleanup (removes all data)
    ./test-environment.sh cleanup full

ENVIRONMENT VARIABLES:
    DEBUG=1                   Enable debug logging
    GOOGLE_API_KEY           API key for AI-enhanced testing

EOF
}

main() {
    if [[ $# -eq 0 ]]; then
        show_help
        exit 0
    fi

    check_dependencies

    local command="$1"
    shift

    case "$command" in
        "start")
            start_environment "${1:-$DEFAULT_PROFILE}"
            ;;
        "stop")
            stop_environment
            ;;
        "restart")
            restart_environment "${1:-$DEFAULT_PROFILE}"
            ;;
        "cleanup")
            cleanup_environment "${1:-false}"
            ;;
        "status")
            show_service_status
            ;;
        "health")
            if wait_for_services "services"; then
                log "SUCCESS" "All services are healthy"
            else
                log "ERROR" "Some services are not healthy"
                exit 1
            fi
            ;;
        "logs")
            show_logs "${1:-all}" "${2:-false}"
            ;;
        "urls")
            show_service_urls
            ;;
        "test")
            if [[ $# -eq 0 ]]; then
                log "ERROR" "Test type is required"
                exit 1
            fi
            run_tests "$1" "${2:-}"
            ;;
        "e2e")
            run_e2e_tests "$*"
            ;;
        "perf")
            run_performance_tests "${1:-load}"
            ;;
        "seed")
            seed_test_data "${1:-all}"
            ;;
        "backup")
            backup_test_data "${1:-}"
            ;;
        "restore")
            if [[ $# -eq 0 ]]; then
                log "ERROR" "Backup name is required"
                exit 1
            fi
            restore_test_data "$1"
            ;;
        "metrics")
            collect_metrics
            ;;
        "report")
            generate_report
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            log "ERROR" "Unknown command: $command"
            log "INFO" "Use './test-environment.sh help' for usage information"
            exit 1
            ;;
    esac
}

# Execute main function with all arguments
main "$@"

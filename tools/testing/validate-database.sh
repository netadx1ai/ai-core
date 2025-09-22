#!/bin/bash

# AI-CORE Database Validation Script
# BUILD/RUN/TEST/FIX validation cycle for Database Agent (Task 10.3)
# Date: 2025-01-11T22:39:18+00:00

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Database connection parameters
POSTGRES_HOST="localhost"
POSTGRES_PORT="5432"
POSTGRES_USER="postgres"
POSTGRES_DB="postgres"

MONGODB_HOST="localhost"
MONGODB_PORT="27017"

CLICKHOUSE_HOST="localhost"
CLICKHOUSE_PORT="8123"

REDIS_HOST="localhost"
REDIS_PORT="6379"

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
    ((TESTS_PASSED++))
    ((TOTAL_TESTS++))
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ((TESTS_FAILED++))
    ((TOTAL_TESTS++))
}

print_test_result() {
    local test_name="$1"
    local success="$2"

    if [ "$success" = "true" ]; then
        print_success "✅ $test_name"
    else
        print_error "❌ $test_name"
    fi
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if port is open
check_port() {
    local host="$1"
    local port="$2"
    local timeout=5

    if command_exists nc; then
        nc -z -w$timeout "$host" "$port" 2>/dev/null
    elif command_exists timeout; then
        timeout $timeout bash -c "</dev/tcp/$host/$port" 2>/dev/null
    else
        # Fallback using telnet if available
        if command_exists telnet; then
            (echo > /dev/tcp/$host/$port) >/dev/null 2>&1
        else
            return 1
        fi
    fi
}

# Function to setup PostgreSQL
setup_postgresql() {
    print_status "Setting up PostgreSQL database and user..."

    # Create AI-CORE database and user
    docker exec ai-core-postgres psql -U postgres -c "
        DO \$\$
        BEGIN
            IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ai_core') THEN
                CREATE ROLE ai_core WITH LOGIN PASSWORD 'password';
            END IF;

            IF NOT EXISTS (SELECT FROM pg_database WHERE datname = 'ai_core_dev') THEN
                CREATE DATABASE ai_core_dev OWNER ai_core;
            END IF;

            GRANT ALL PRIVILEGES ON DATABASE ai_core_dev TO ai_core;
        END
        \$\$;" 2>/dev/null || {
        print_error "Failed to setup PostgreSQL database"
        return 1
    }

    # Create basic schema
    docker exec ai-core-postgres psql -U ai_core -d ai_core_dev -c "
        CREATE TABLE IF NOT EXISTS health_check (
            id SERIAL PRIMARY KEY,
            status VARCHAR(50) NOT NULL,
            checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );

        INSERT INTO health_check (status) VALUES ('database_setup_complete')
        ON CONFLICT DO NOTHING;" 2>/dev/null || {
        print_warning "Could not create test schema, but user/database setup succeeded"
    }

    print_success "PostgreSQL setup complete"
}

# Function to validate database connections
validate_database_connections() {
    print_status "=== DATABASE CONNECTION VALIDATION ==="

    # PostgreSQL Connection Test
    print_status "Testing PostgreSQL connection..."
    if check_port "$POSTGRES_HOST" "$POSTGRES_PORT"; then
        if docker exec ai-core-postgres pg_isready -U postgres >/dev/null 2>&1; then
            print_test_result "PostgreSQL connection" "true"
        else
            print_test_result "PostgreSQL connection" "false"
        fi
    else
        print_test_result "PostgreSQL connection" "false"
    fi

    # MongoDB Connection Test
    print_status "Testing MongoDB connection..."
    if check_port "$MONGODB_HOST" "$MONGODB_PORT"; then
        if docker exec ai-core-mongodb mongosh --eval "db.adminCommand('ping')" >/dev/null 2>&1; then
            print_test_result "MongoDB connection" "true"
        else
            print_test_result "MongoDB connection" "false"
        fi
    else
        print_test_result "MongoDB connection" "false"
    fi

    # ClickHouse Connection Test
    print_status "Testing ClickHouse connection..."
    if check_port "$CLICKHOUSE_HOST" "$CLICKHOUSE_PORT"; then
        if curl -s "http://$CLICKHOUSE_HOST:$CLICKHOUSE_PORT/ping" | grep -q "Ok" >/dev/null 2>&1; then
            print_test_result "ClickHouse connection" "true"
        else
            print_test_result "ClickHouse connection" "false"
        fi
    else
        print_test_result "ClickHouse connection" "false"
    fi

    # Redis Connection Test
    print_status "Testing Redis connection..."
    if check_port "$REDIS_HOST" "$REDIS_PORT"; then
        if docker exec ai-core-redis redis-cli ping | grep -q "PONG" >/dev/null 2>&1; then
            print_test_result "Redis connection" "true"
        else
            print_test_result "Redis connection" "false"
        fi
    else
        print_test_result "Redis connection" "false"
    fi
}

# Function to validate database schemas
validate_database_schemas() {
    print_status "=== DATABASE SCHEMA VALIDATION ==="

    # PostgreSQL Schema Validation
    print_status "Validating PostgreSQL schema..."
    if docker exec ai-core-postgres psql -U postgres -d ai_core_dev -c "\dt" >/dev/null 2>&1; then
        print_test_result "PostgreSQL schema access" "true"
    else
        print_test_result "PostgreSQL schema access" "false"
    fi

    # MongoDB Collections Test
    print_status "Testing MongoDB collections..."
    if docker exec ai-core-mongodb mongosh automation_platform --eval "
        db.test_collection.insertOne({test: 'validation', timestamp: new Date()});
        db.test_collection.deleteOne({test: 'validation'});
    " >/dev/null 2>&1; then
        print_test_result "MongoDB collections" "true"
    else
        print_test_result "MongoDB collections" "false"
    fi

    # ClickHouse Database Test
    print_status "Testing ClickHouse database..."
    if curl -s "http://$CLICKHOUSE_HOST:$CLICKHOUSE_PORT/" -d "SHOW DATABASES" | grep -q "default" >/dev/null 2>&1; then
        print_test_result "ClickHouse database" "true"
    else
        print_test_result "ClickHouse database" "false"
    fi

    # Redis Key Operations
    print_status "Testing Redis operations..."
    if docker exec ai-core-redis redis-cli SET test_key "validation" >/dev/null 2>&1 && \
       docker exec ai-core-redis redis-cli GET test_key >/dev/null 2>&1 && \
       docker exec ai-core-redis redis-cli DEL test_key >/dev/null 2>&1; then
        print_test_result "Redis operations" "true"
    else
        print_test_result "Redis operations" "false"
    fi
}

# Function to run Rust database tests
run_rust_database_tests() {
    print_status "=== RUST DATABASE TESTS ==="

    cd "$PROJECT_ROOT/src/database"

    # Build test
    print_status "Building database crate..."
    if cargo build --release --features "postgres clickhouse mongodb redis" >/dev/null 2>&1; then
        print_test_result "Database crate build" "true"
    else
        print_test_result "Database crate build" "false"
    fi

    # Unit tests
    print_status "Running unit tests..."
    if cargo test --lib --features "postgres clickhouse mongodb redis" >/dev/null 2>&1; then
        print_test_result "Database unit tests" "true"
    else
        print_test_result "Database unit tests" "false"
    fi

    # Connection validation with proper credentials
    print_status "Testing database connections with Rust..."
    export DATABASE_URL="postgresql://ai_core:password@localhost:5432/ai_core_dev"
    export MONGODB_URL="mongodb://localhost:27017/automation_platform"
    export CLICKHOUSE_URL="http://localhost:8123"
    export REDIS_URL="redis://localhost:6379"

    if timeout 30 cargo run --example basic_usage --features "postgres clickhouse mongodb redis" 2>/dev/null; then
        print_test_result "Rust database integration" "true"
    else
        print_test_result "Rust database integration" "false"
    fi

    cd "$PROJECT_ROOT"
}

# Function to run performance benchmarks
run_performance_benchmarks() {
    print_status "=== PERFORMANCE BENCHMARKS ==="

    cd "$PROJECT_ROOT/src/database"

    # Run lightweight performance tests
    print_status "Running performance benchmarks..."
    if timeout 60 cargo bench --features "postgres clickhouse mongodb redis" >/dev/null 2>&1; then
        print_test_result "Performance benchmarks" "true"
    else
        print_test_result "Performance benchmarks" "false"
    fi

    cd "$PROJECT_ROOT"
}

# Function to validate database migration system
validate_migration_system() {
    print_status "=== MIGRATION SYSTEM VALIDATION ==="

    cd "$PROJECT_ROOT/src/database"

    # Test migration functionality
    print_status "Testing migration system..."
    if cargo test migration --features "postgres clickhouse mongodb redis" >/dev/null 2>&1; then
        print_test_result "Migration system tests" "true"
    else
        print_test_result "Migration system tests" "false"
    fi

    cd "$PROJECT_ROOT"
}

# Function to check code quality
check_code_quality() {
    print_status "=== CODE QUALITY VALIDATION ==="

    cd "$PROJECT_ROOT/src/database"

    # Clippy linting
    print_status "Running clippy linting..."
    if cargo clippy --features "postgres clickhouse mongodb redis" -- -D warnings >/dev/null 2>&1; then
        print_test_result "Clippy linting" "true"
    else
        print_test_result "Clippy linting" "false"
    fi

    # Format check
    print_status "Checking code formatting..."
    if cargo fmt --check >/dev/null 2>&1; then
        print_test_result "Code formatting" "true"
    else
        print_test_result "Code formatting" "false"
    fi

    # Security audit
    print_status "Running security audit..."
    if cargo audit >/dev/null 2>&1; then
        print_test_result "Security audit" "true"
    else
        print_test_result "Security audit" "false"
    fi

    cd "$PROJECT_ROOT"
}

# Main execution function
main() {
    print_status "🔍 AI-CORE Database Agent BUILD/RUN/TEST/FIX Validation"
    print_status "📅 Started at: $(date)"
    print_status "=============================================="

    # Ensure we're in the project root
    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        print_error "Not in AI-CORE project root directory"
        exit 1
    fi

    # Setup databases
    setup_postgresql

    # Run validation phases
    validate_database_connections
    validate_database_schemas
    run_rust_database_tests
    validate_migration_system
    run_performance_benchmarks
    check_code_quality

    # Print final results
    print_status "=============================================="
    print_status "📊 VALIDATION COMPLETE"
    print_status "✅ Tests Passed: $TESTS_PASSED"
    print_status "❌ Tests Failed: $TESTS_FAILED"
    print_status "📈 Total Tests: $TOTAL_TESTS"

    if [ $TESTS_FAILED -eq 0 ]; then
        print_success "🎉 ALL TESTS PASSED - Database Agent ready for production!"
        print_status "📝 Task 10.3 Database Agent BUILD/RUN/TEST/FIX validation: COMPLETED ✅"
        exit 0
    else
        local success_rate=$(( (TESTS_PASSED * 100) / TOTAL_TESTS ))
        if [ $success_rate -ge 80 ]; then
            print_warning "⚠️  Most tests passed ($success_rate%) - Database Agent functional with minor issues"
            print_status "📝 Task 10.3 Database Agent BUILD/RUN/TEST/FIX validation: MOSTLY COMPLETE ⚠️"
            exit 1
        else
            print_error "❌ Significant issues detected ($success_rate% success) - Database Agent needs fixes"
            print_status "📝 Task 10.3 Database Agent BUILD/RUN/TEST/FIX validation: FAILED ❌"
            exit 2
        fi
    fi
}

# Parse command line arguments
case "${1:-}" in
    --help|-h)
        echo "AI-CORE Database Validation Script"
        echo ""
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --help, -h          Show this help message"
        echo "  --connections-only  Only test database connections"
        echo "  --tests-only        Only run Rust tests"
        echo "  --performance-only  Only run performance benchmarks"
        echo ""
        echo "This script validates the Database Agent implementation"
        echo "according to Task 10.3 BUILD/RUN/TEST/FIX requirements."
        exit 0
        ;;
    --connections-only)
        validate_database_connections
        exit 0
        ;;
    --tests-only)
        run_rust_database_tests
        exit 0
        ;;
    --performance-only)
        run_performance_benchmarks
        exit 0
        ;;
    "")
        main
        ;;
    *)
        print_error "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac

#!/bin/bash

# AI-PLATFORM Database Setup Script
# This script sets up the complete database environment for development

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCKER_DIR="$PROJECT_ROOT/infrastructure/docker"
VOLUMES_DIR="$DOCKER_DIR/volumes"

# Default values
SKIP_DOCKER=false
SKIP_INIT=false
VERBOSE=false
CLEAN=false

# Function to print colored output
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

# Function to show usage
show_usage() {
    cat << EOF
AI-PLATFORM Database Setup Script

Usage: $0 [OPTIONS]

Options:
    -h, --help              Show this help message
    -s, --skip-docker       Skip Docker container setup
    -i, --skip-init         Skip database initialization
    -c, --clean             Clean existing data and start fresh
    -v, --verbose           Enable verbose output

Examples:
    $0                      # Full setup with default options
    $0 --clean              # Clean setup (removes existing data)
    $0 --skip-docker        # Only initialize databases (assumes containers are running)
    $0 --verbose            # Show detailed output

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_usage
            exit 0
            ;;
        -s|--skip-docker)
            SKIP_DOCKER=true
            shift
            ;;
        -i|--skip-init)
            SKIP_INIT=true
            shift
            ;;
        -c|--clean)
            CLEAN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."

    local missing_deps=()

    if ! command_exists docker; then
        missing_deps+=("docker")
    fi

    if ! command_exists docker-compose; then
        missing_deps+=("docker-compose")
    fi

    if ! command_exists psql; then
        print_warning "PostgreSQL client (psql) not found - optional for testing connections"
    fi

    if ! command_exists mongosh; then
        print_warning "MongoDB shell (mongosh) not found - optional for testing connections"
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_error "Missing required dependencies: ${missing_deps[*]}"
        print_error "Please install the missing dependencies and try again"
        exit 1
    fi

    print_success "All required dependencies found"
}

# Function to create directory structure
create_directories() {
    print_status "Creating directory structure..."

    local dirs=(
        "$VOLUMES_DIR"
        "$VOLUMES_DIR/postgres"
        "$VOLUMES_DIR/mongodb"
        "$VOLUMES_DIR/mongodb_config"
        "$VOLUMES_DIR/clickhouse"
        "$VOLUMES_DIR/clickhouse_logs"
        "$VOLUMES_DIR/redis"
        "$VOLUMES_DIR/pgadmin"
        "$VOLUMES_DIR/backups"
        "$DOCKER_DIR/postgres/init"
        "$DOCKER_DIR/postgres/conf"
        "$DOCKER_DIR/mongodb/init"
        "$DOCKER_DIR/mongodb/conf"
        "$DOCKER_DIR/clickhouse/config"
        "$DOCKER_DIR/clickhouse/users"
        "$DOCKER_DIR/clickhouse/init"
        "$DOCKER_DIR/redis/conf"
        "$DOCKER_DIR/pgadmin"
        "$DOCKER_DIR/monitoring/postgres-exporter"
        "$DOCKER_DIR/scripts"
    )

    for dir in "${dirs[@]}"; do
        if [ ! -d "$dir" ]; then
            mkdir -p "$dir"
            if [ $VERBOSE = true ]; then
                print_status "Created directory: $dir"
            fi
        fi
    done

    print_success "Directory structure created"
}

# Function to clean existing data
clean_data() {
    if [ $CLEAN = true ]; then
        print_warning "Cleaning existing database data..."
        read -p "Are you sure you want to delete all existing database data? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_status "Stopping existing containers..."
            cd "$DOCKER_DIR" && docker-compose -f docker-compose.databases.yml down -v 2>/dev/null || true

            print_status "Removing data volumes..."
            sudo rm -rf "$VOLUMES_DIR"/* 2>/dev/null || true

            print_success "Data cleaned successfully"
        else
            print_status "Skipping data cleanup"
        fi
    fi
}

# Function to create configuration files
create_config_files() {
    print_status "Creating configuration files..."

    # PostgreSQL configuration
    cat > "$DOCKER_DIR/postgres/conf/postgresql.conf" << 'EOF'
# PostgreSQL configuration for AI-PLATFORM development
listen_addresses = '*'
max_connections = 200
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB
maintenance_work_mem = 64MB
random_page_cost = 1.1
effective_io_concurrency = 200
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100

# Logging
log_statement = 'all'
log_duration = on
log_line_prefix = '%t [%p]: [%l-1] user=%u,db=%d,app=%a,client=%h '
log_min_duration_statement = 1000

# Extensions
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.track = all
pg_stat_statements.save = on
EOF

    # MongoDB configuration
    cat > "$DOCKER_DIR/mongodb/conf/mongod.conf" << 'EOF'
storage:
  dbPath: /data/db
  journal:
    enabled: true

systemLog:
  destination: file
  logAppend: true
  path: /var/log/mongodb/mongod.log
  logRotate: reopen

net:
  port: 27017
  bindIp: 0.0.0.0

processManagement:
  timeZoneInfo: /usr/share/zoneinfo

security:
  authorization: enabled

operationProfiling:
  slowOpThresholdMs: 1000
  mode: slowOp

setParameter:
  diagnosticDataCollectionEnabled: false
EOF

    # ClickHouse configuration
    cat > "$DOCKER_DIR/clickhouse/config/config.xml" << 'EOF'
<?xml version="1.0"?>
<clickhouse>
    <http_port>8123</http_port>
    <tcp_port>9000</tcp_port>
    <mysql_port>9004</mysql_port>
    <postgresql_port>9005</postgresql_port>

    <listen_host>0.0.0.0</listen_host>

    <max_connections>4096</max_connections>
    <keep_alive_timeout>3</keep_alive_timeout>
    <max_concurrent_queries>100</max_concurrent_queries>
    <uncompressed_cache_size>8589934592</uncompressed_cache_size>
    <mark_cache_size>5368709120</mark_cache_size>

    <path>/var/lib/clickhouse/</path>
    <tmp_path>/var/lib/clickhouse/tmp/</tmp_path>
    <user_files_path>/var/lib/clickhouse/user_files/</user_files_path>
    <users_config>users.xml</users_config>

    <default_profile>default</default_profile>
    <default_database>default</default_database>

    <timezone>UTC</timezone>
    <umask>022</umask>

    <mlock_executable>false</mlock_executable>

    <remote_servers>
        <test_shard_localhost>
            <shard>
                <replica>
                    <host>localhost</host>
                    <port>9000</port>
                </replica>
            </shard>
        </test_shard_localhost>
    </remote_servers>

    <zookeeper incl="zookeeper-servers" optional="true" />
    <macros incl="macros" optional="true" />

    <builtin_dictionaries_reload_interval>3600</builtin_dictionaries_reload_interval>

    <max_session_timeout>3600</max_session_timeout>
    <default_session_timeout>60</default_session_timeout>

    <query_log>
        <database>system</database>
        <table>query_log</table>
        <partition_by>toYYYYMM(event_date)</partition_by>
        <flush_interval_milliseconds>7500</flush_interval_milliseconds>
    </query_log>

    <trace_log>
        <database>system</database>
        <table>trace_log</table>
        <partition_by>toYYYYMM(event_date)</partition_by>
        <flush_interval_milliseconds>7500</flush_interval_milliseconds>
    </trace_log>
</clickhouse>
EOF

    # ClickHouse users configuration
    cat > "$DOCKER_DIR/clickhouse/users/users.xml" << 'EOF'
<?xml version="1.0"?>
<clickhouse>
    <profiles>
        <default>
            <max_memory_usage>10000000000</max_memory_usage>
            <use_uncompressed_cache>0</use_uncompressed_cache>
            <load_balancing>random</load_balancing>
        </default>

        <readonly>
            <readonly>1</readonly>
        </readonly>
    </profiles>

    <users>
        <default>
            <password></password>
            <networks incl="networks" replace="replace">
                <ip>::/0</ip>
            </networks>
            <profile>default</profile>
            <quota>default</quota>
        </default>

        <ai_core_clickhouse>
            <password>ai_core_clickhouse_password</password>
            <networks>
                <ip>::/0</ip>
            </networks>
            <profile>default</profile>
            <quota>default</quota>
        </ai_core_clickhouse>
    </users>

    <quotas>
        <default>
            <interval>
                <duration>3600</duration>
                <queries>0</queries>
                <errors>0</errors>
                <result_rows>0</result_rows>
                <read_rows>0</read_rows>
                <execution_time>0</execution_time>
            </interval>
        </default>
    </quotas>
</clickhouse>
EOF

    # Redis configuration
    cat > "$DOCKER_DIR/redis/conf/redis.conf" << 'EOF'
# Redis configuration for AI-PLATFORM development

# Network
bind 0.0.0.0
port 6379
tcp-backlog 511
timeout 0
tcp-keepalive 300

# General
daemonize no
pidfile /var/run/redis_6379.pid
loglevel notice
logfile ""
databases 16

# Snapshotting
save 900 1
save 300 10
save 60 10000
stop-writes-on-bgsave-error yes
rdbcompression yes
rdbchecksum yes
dbfilename dump.rdb
dir /data

# Replication
# masterauth <master-password>
# requirepass <password>

# Security
# rename-command FLUSHDB ""
# rename-command FLUSHALL ""
# rename-command DEBUG ""

# Limits
maxclients 10000
maxmemory 2gb
maxmemory-policy allkeys-lru

# Append only file
appendonly yes
appendfilename "appendonly.aof"
appendfsync everysec
no-appendfsync-on-rewrite no
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb

# Lua scripting
lua-time-limit 5000

# Slow log
slowlog-log-slower-than 10000
slowlog-max-len 128

# Event notification
notify-keyspace-events ""

# Advanced config
hash-max-ziplist-entries 512
hash-max-ziplist-value 64
list-max-ziplist-size -2
list-compress-depth 0
set-max-intset-entries 512
zset-max-ziplist-entries 128
zset-max-ziplist-value 64
hll-sparse-max-bytes 3000
stream-node-max-bytes 4096
stream-node-max-entries 100
activerehashing yes
client-output-buffer-limit normal 0 0 0
client-output-buffer-limit replica 256mb 64mb 60
client-output-buffer-limit pubsub 32mb 8mb 60
hz 10
dynamic-hz yes
aof-rewrite-incremental-fsync yes
rdb-save-incremental-fsync yes
EOF

    # pgAdmin servers configuration
    cat > "$DOCKER_DIR/pgadmin/servers.json" << 'EOF'
{
  "Servers": {
    "1": {
      "Name": "AI-PLATFORM PostgreSQL",
      "Group": "Development",
      "Host": "postgres",
      "Port": 5432,
      "MaintenanceDB": "ai_core",
      "Username": "ai_core_user",
      "SSLMode": "prefer",
      "SSLCert": "<STORAGE_DIR>/.postgresql/postgresql.crt",
      "SSLKey": "<STORAGE_DIR>/.postgresql/postgresql.key",
      "SSLCompression": 0,
      "Timeout": 10,
      "UseSSHTunnel": 0,
      "TunnelPort": "22",
      "TunnelAuthentication": 0
    }
  }
}
EOF

    # Backup script
    cat > "$DOCKER_DIR/scripts/backup.sh" << 'EOF'
#!/bin/bash

# Database backup script for AI-PLATFORM
set -e

BACKUP_DIR="/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR/$DATE"

echo "Starting backup process at $(date)"

# PostgreSQL backup
echo "Backing up PostgreSQL..."
PGPASSWORD=$POSTGRES_PASSWORD pg_dump -h $POSTGRES_HOST -U $POSTGRES_USER $POSTGRES_DB > "$BACKUP_DIR/$DATE/postgres_backup.sql"
gzip "$BACKUP_DIR/$DATE/postgres_backup.sql"

# MongoDB backup
echo "Backing up MongoDB..."
mongodump --host $MONGO_HOST --username $MONGO_USER --password $MONGO_PASSWORD --db $MONGO_DB --out "$BACKUP_DIR/$DATE/mongodb_backup"
tar -czf "$BACKUP_DIR/$DATE/mongodb_backup.tar.gz" -C "$BACKUP_DIR/$DATE" mongodb_backup
rm -rf "$BACKUP_DIR/$DATE/mongodb_backup"

# Clean old backups
echo "Cleaning old backups..."
find "$BACKUP_DIR" -type d -mtime +$BACKUP_RETENTION_DAYS -exec rm -rf {} + 2>/dev/null || true

echo "Backup completed at $(date)"
echo "Backup saved to: $BACKUP_DIR/$DATE"
EOF

    chmod +x "$DOCKER_DIR/scripts/backup.sh"

    # Monitoring queries for PostgreSQL exporter
    cat > "$DOCKER_DIR/monitoring/postgres-exporter/queries.yaml" << 'EOF'
pg_replication:
  query: "SELECT CASE WHEN NOT pg_is_in_recovery() THEN 0 ELSE GREATEST (0, EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp()))) END AS lag"
  master: true
  metrics:
    - lag:
        usage: "GAUGE"
        description: "Replication lag behind master in seconds"

pg_postmaster:
  query: "SELECT pg_postmaster_start_time as start_time_seconds from pg_postmaster_start_time()"
  master: true
  metrics:
    - start_time_seconds:
        usage: "GAUGE"
        description: "Time at which postmaster started"

pg_stat_user_tables:
  query: |
    SELECT
      current_database() datname,
      schemaname,
      relname,
      seq_scan,
      seq_tup_read,
      idx_scan,
      idx_tup_fetch,
      n_tup_ins,
      n_tup_upd,
      n_tup_del,
      n_tup_hot_upd,
      n_live_tup,
      n_dead_tup,
      n_mod_since_analyze,
      COALESCE(last_vacuum, '1970-01-01Z') as last_vacuum,
      COALESCE(last_autovacuum, '1970-01-01Z') as last_autovacuum,
      COALESCE(last_analyze, '1970-01-01Z') as last_analyze,
      COALESCE(last_autoanalyze, '1970-01-01Z') as last_autoanalyze,
      vacuum_count,
      autovacuum_count,
      analyze_count,
      autoanalyze_count
    FROM pg_stat_user_tables
  metrics:
    - datname:
        usage: "LABEL"
        description: "Name of current database"
    - schemaname:
        usage: "LABEL"
        description: "Name of the schema that this table is in"
    - relname:
        usage: "LABEL"
        description: "Name of this table"
    - seq_scan:
        usage: "COUNTER"
        description: "Number of sequential scans initiated on this table"
    - seq_tup_read:
        usage: "COUNTER"
        description: "Number of live rows fetched by sequential scans"
    - idx_scan:
        usage: "COUNTER"
        description: "Number of index scans initiated on this table"
    - idx_tup_fetch:
        usage: "COUNTER"
        description: "Number of live rows fetched by index scans"
    - n_tup_ins:
        usage: "COUNTER"
        description: "Number of rows inserted"
    - n_tup_upd:
        usage: "COUNTER"
        description: "Number of rows updated"
    - n_tup_del:
        usage: "COUNTER"
        description: "Number of rows deleted"
    - n_tup_hot_upd:
        usage: "COUNTER"
        description: "Number of rows HOT updated"
    - n_live_tup:
        usage: "GAUGE"
        description: "Estimated number of live rows"
    - n_dead_tup:
        usage: "GAUGE"
        description: "Estimated number of dead rows"
    - n_mod_since_analyze:
        usage: "GAUGE"
        description: "Estimated number of rows changed since last analyze"
    - last_vacuum:
        usage: "GAUGE"
        description: "Last time at which this table was manually vacuumed"
    - last_autovacuum:
        usage: "GAUGE"
        description: "Last time at which this table was vacuumed by the autovacuum daemon"
    - last_analyze:
        usage: "GAUGE"
        description: "Last time at which this table was manually analyzed"
    - last_autoanalyze:
        usage: "GAUGE"
        description: "Last time at which this table was analyzed by the autovacuum daemon"
    - vacuum_count:
        usage: "COUNTER"
        description: "Number of times this table has been manually vacuumed"
    - autovacuum_count:
        usage: "COUNTER"
        description: "Number of times this table has been vacuumed by the autovacuum daemon"
    - analyze_count:
        usage: "COUNTER"
        description: "Number of times this table has been manually analyzed"
    - autoanalyze_count:
        usage: "COUNTER"
        description: "Number of times this table has been analyzed by the autovacuum daemon"
EOF

    print_success "Configuration files created"
}

# Function to start Docker containers
start_containers() {
    if [ $SKIP_DOCKER = true ]; then
        print_status "Skipping Docker container setup"
        return
    fi

    print_status "Starting database containers..."

    cd "$DOCKER_DIR"

    # Pull latest images
    print_status "Pulling Docker images..."
    docker-compose -f docker-compose.databases.yml pull

    # Start containers
    print_status "Starting containers..."
    docker-compose -f docker-compose.databases.yml up -d

    print_success "Database containers started"
}

# Function to wait for services to be ready
wait_for_services() {
    print_status "Waiting for services to be ready..."

    local max_attempts=60
    local attempt=0

    # Wait for PostgreSQL
    print_status "Waiting for PostgreSQL..."
    while ! docker exec AI-PLATFORM-postgres pg_isready -U ai_core_user -d ai_core >/dev/null 2>&1; do
        attempt=$((attempt + 1))
        if [ $attempt -eq $max_attempts ]; then
            print_error "PostgreSQL failed to start within expected time"
            exit 1
        fi
        sleep 2
    done

    # Wait for MongoDB
    print_status "Waiting for MongoDB..."
    attempt=0
    while ! docker exec AI-PLATFORM-mongodb mongosh --eval "db.adminCommand('ping')" >/dev/null 2>&1; do
        attempt=$((attempt + 1))
        if [ $attempt -eq $max_attempts ]; then
            print_error "MongoDB failed to start within expected time"
            exit 1
        fi
        sleep 2
    done

    # Wait for ClickHouse
    print_status "Waiting for ClickHouse..."
    attempt=0
    while ! curl -s http://localhost:8123/ping >/dev/null 2>&1; do
        attempt=$((attempt + 1))
        if [ $attempt -eq $max_attempts ]; then
            print_error "ClickHouse failed to start within expected time"
            exit 1
        fi
        sleep 2
    done

    # Wait for Redis
    print_status "Waiting for Redis..."
    attempt=0
    while ! docker exec AI-PLATFORM-redis redis-cli ping >/dev/null 2>&1; do
        attempt=$((attempt + 1))
        if [ $attempt -eq $max_attempts ]; then
            print_error "Redis failed to start within expected time"
            exit 1
        fi
        sleep 2
    done

    print_success "All services are ready"
}

# Function to initialize databases
initialize_databases() {
    if [ $SKIP_INIT = true ]; then
        print_status "Skipping database initialization"
        return
    fi

    print_status "Initializing databases..."

    # Initialize PostgreSQL
    print_status "Initializing PostgreSQL..."
    docker exec -i AI-PLATFORM-postgres psql -U ai_core_user -d ai_core << 'EOF'
-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";

-- Create test user
INSERT INTO pg_user (usename, usesysid, usecreatedb, usesuper, usecatupd, passwd, valuntil, useconfig)
SELECT 'test_user', 16384, 'f', 'f', 'f', 'md5' || md5('test_password' || 'test_user'), null, null
WHERE NOT EXISTS (SELECT 1 FROM pg_user WHERE usename = 'test_user');

-- Show version
SELECT version();
EOF

    # Initialize MongoDB
    print_status "Initializing MongoDB..."
    docker exec -i AI-PLATFORM-mongodb mongosh automation_platform << 'EOF'
// Create application user
db.createUser({
  user: "ai_core_app",
  pwd: "ai_core_app_password",
  roles: [
    { role: "readWrite", db: "automation_platform" },
    { role: "dbAdmin", db: "automation_platform" }
  ]
});

// Create indexes for performance
db.content_items.createIndex({ "content_type": 1, "status": 1 });
db.content_items.createIndex({ "created_at": -1 });
db.campaigns.createIndex({ "status": 1, "created_at": -1 });

// Show status
db.runCommand({ serverStatus: 1 });
EOF

    # Initialize ClickHouse
    print_status "Initializing ClickHouse..."
    docker exec -i AI-PLATFORM-clickhouse clickhouse-client --query="
        CREATE DATABASE IF NOT EXISTS analytics;

        -- Test table
        CREATE TABLE IF NOT EXISTS analytics.test_events (
            timestamp DateTime64(3),
            event_id UUID,
            event_type String,
            properties Map(String, String)
        ) ENGINE = MergeTree()
        ORDER BY timestamp;

        -- Show version
        SELECT version();
    "

    # Initialize Redis
    print_status "Initializing Redis..."
    docker exec -i AI-PLATFORM-redis redis-cli << 'EOF'
# Set up basic cache configuration
SET cache:config:default:ttl 3600
SET cache:config:users:ttl 1800

# Test connection
PING

# Show info
INFO server
EOF

    print_success "Database initialization completed"
}

# Function to test connections
test_connections() {
    print_status "Testing database connections..."

    # Test PostgreSQL
    if docker exec AI-PLATFORM-postgres pg_isready -U ai_core_user -d ai_core >/dev/null 2>&1; then
        print_success "PostgreSQL connection: OK"
    else
        print_error "PostgreSQL connection: FAILED"
    fi

    # Test MongoDB
    if docker exec AI-PLATFORM-mongodb mongosh --eval "db.adminCommand('ping')" >/dev/null 2>&1; then
        print_success "MongoDB connection: OK"
    else
        print_error "MongoDB connection: FAILED"
    fi

    # Test ClickHouse
    if curl -s http://localhost:8123/ping >/dev/null 2>&1; then
        print_success "ClickHouse connection: OK"
    else
        print_error "ClickHouse connection: FAILED"
    fi

    # Test Redis
    if docker exec AI-PLATFORM-redis redis-cli ping >/dev/null 2>&1; then
        print_success "Redis connection: OK"
    else
        print_error "Redis connection: FAILED"
    fi
}

# Function to show service information
show_service_info() {
    print_status "Database services information:"
    echo
    echo "🐘 PostgreSQL:"
    echo "   URL: postgresql://ai_core_user:ai_core_password@localhost:5432/ai_core"
    echo "   Admin UI: http://localhost:8080 (admin@aicore.local / pgadmin_password)"
    echo
    echo "🍃 MongoDB:"
    echo "   URL: mongodb://ai_core_admin:ai_core_mongo_password@localhost:27017/automation_platform"
    echo "   Admin UI: http://localhost:8082 (admin / mongo_admin_password)"
    echo
    echo "⚡ ClickHouse:"
    echo "   HTTP: http://localhost:8123"
    echo "   Native: localhost:9000"
    echo "   Admin UI: http://localhost:8083"
    echo "   User: ai_core_clickhouse / ai_core_clickhouse_password"
    echo
    echo "🔴 Redis:"
    echo "   URL: redis://localhost:6379"
    echo "   Admin UI: http://localhost:8081 (admin / redis_admin_password)"
    echo
    echo "📊 Monitoring Exporters:"
    echo "   PostgreSQL: http://localhost:9187/metrics"
    echo "   MongoDB: http://localhost:9216/metrics"
    echo "   Redis: http://localhost:9121/metrics"
    echo
    echo "Volumes located at: $VOLUMES_DIR"
    echo
    print_success "Setup completed successfully!"
}

# Main execution
main() {
    echo "======================================"
    echo "AI-PLATFORM Database Setup Script"
    echo "======================================"
    echo

    check_prerequisites
    create_directories
    clean_data
    create_config_files
    start_containers

    if [ $SKIP_DOCKER = false ]; then
        wait_for_services
    fi

    initialize_databases
    test_connections
    show_service_info
}

# Run main function
main "$@"

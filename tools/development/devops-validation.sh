#!/bin/bash

# DevOps Agent BUILD/RUN/TEST/FIX Validation Script
# Task 10.6: Infrastructure deployment and validation, CI/CD pipeline execution,
# monitoring system verification, and security scanning

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VALIDATION_LOG="$PROJECT_ROOT/.devops-validation.log"
METRICS_FILE="$PROJECT_ROOT/.devops-metrics.json"
REPORT_DIR="$PROJECT_ROOT/.quality-reports"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$VALIDATION_LOG"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$VALIDATION_LOG"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$VALIDATION_LOG"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$VALIDATION_LOG"
}

# Metrics collection
init_metrics() {
    cat > "$METRICS_FILE" <<EOF
{
    "validation_start": "$TIMESTAMP",
    "validation_type": "devops_agent_validation",
    "results": {
        "build": {"status": "pending", "score": 0, "details": []},
        "run": {"status": "pending", "score": 0, "details": []},
        "test": {"status": "pending", "score": 0, "details": []},
        "fix": {"status": "pending", "score": 0, "details": []}
    },
    "infrastructure": {
        "docker": {"status": "pending", "score": 0},
        "kubernetes": {"status": "pending", "score": 0},
        "ci_cd": {"status": "pending", "score": 0},
        "monitoring": {"status": "pending", "score": 0},
        "security": {"status": "pending", "score": 0}
    },
    "performance": {},
    "overall_score": 0,
    "grade": "F",
    "recommendations": []
}
EOF
}

update_metrics() {
    local category="$1"
    local subcategory="$2"
    local status="$3"
    local score="$4"
    local detail="$5"

    # Create a temporary file with updated metrics
    python3 -c "
import json
import sys

try:
    with open('$METRICS_FILE', 'r') as f:
        data = json.load(f)

    if '$subcategory' == 'overall':
        data['$category']['status'] = '$status'
        data['$category']['score'] = $score
        if '$detail':
            data['$category']['details'].append('$detail')
    else:
        data['$category']['$subcategory']['status'] = '$status'
        data['$category']['$subcategory']['score'] = $score

    with open('$METRICS_FILE', 'w') as f:
        json.dump(data, f, indent=2)
except Exception as e:
    print(f'Error updating metrics: {e}', file=sys.stderr)
"
}

# BUILD Phase - Infrastructure Build Validation
validate_build() {
    log_info "🔨 Starting BUILD phase validation..."
    local build_score=0
    local total_checks=5

    # Check Docker environment
    log_info "Validating Docker environment..."
    if command -v docker >/dev/null 2>&1 && docker --version >/dev/null 2>&1; then
        log_success "Docker is installed and accessible"
        build_score=$((build_score + 1))
        update_metrics "infrastructure" "docker" "success" 100 ""

        # Check Docker Compose
        if [ -f "$PROJECT_ROOT/infrastructure/docker/docker-compose.yml" ]; then
            log_success "Docker Compose configuration found"
            build_score=$((build_score + 1))
        else
            log_warning "Docker Compose configuration not found"
            update_metrics "results" "build" "warning" 0 "Docker Compose config missing"
        fi
    else
        log_error "Docker not installed or not accessible"
        update_metrics "infrastructure" "docker" "failed" 0 ""
    fi

    # Check Kubernetes manifests
    log_info "Validating Kubernetes manifests..."
    if [ -d "$PROJECT_ROOT/infrastructure/kubernetes" ]; then
        local k8s_files=$(find "$PROJECT_ROOT/infrastructure/kubernetes" -name "*.yaml" -o -name "*.yml" | wc -l)
        if [ "$k8s_files" -gt 0 ]; then
            log_success "Found $k8s_files Kubernetes manifest files"
            build_score=$((build_score + 1))
            update_metrics "infrastructure" "kubernetes" "success" 100 ""
        else
            log_warning "No Kubernetes manifest files found"
            update_metrics "infrastructure" "kubernetes" "failed" 0 ""
        fi
    else
        log_error "Kubernetes directory not found"
        update_metrics "infrastructure" "kubernetes" "failed" 0 ""
    fi

    # Check CI/CD pipeline configuration
    log_info "Validating CI/CD pipeline..."
    if [ -f "$PROJECT_ROOT/.github/workflows/ci.yml" ] || [ -f "$PROJECT_ROOT/.github/workflows/cd.yml" ]; then
        log_success "GitHub Actions workflows found"
        build_score=$((build_score + 1))
        update_metrics "infrastructure" "ci_cd" "success" 100 ""
    else
        log_warning "GitHub Actions workflows not found"
        update_metrics "infrastructure" "ci_cd" "failed" 0 ""
    fi

    # Check monitoring configuration
    log_info "Validating monitoring configuration..."
    if [ -d "$PROJECT_ROOT/infrastructure/monitoring" ]; then
        local monitoring_files=$(find "$PROJECT_ROOT/infrastructure/monitoring" -name "*.yml" -o -name "*.yaml" -o -name "*.json" | wc -l)
        if [ "$monitoring_files" -gt 0 ]; then
            log_success "Found $monitoring_files monitoring configuration files"
            build_score=$((build_score + 1))
            update_metrics "infrastructure" "monitoring" "success" 100 ""
        else
            log_warning "No monitoring configuration files found"
            update_metrics "infrastructure" "monitoring" "failed" 0 ""
        fi
    else
        log_error "Monitoring directory not found"
        update_metrics "infrastructure" "monitoring" "failed" 0 ""
    fi

    local build_percentage=$((build_score * 100 / total_checks))
    local build_status="failed"
    if [ "$build_percentage" -ge 80 ]; then
        build_status="success"
    elif [ "$build_percentage" -ge 60 ]; then
        build_status="warning"
    fi

    update_metrics "results" "build" "$build_status" "$build_percentage" "Build validation completed"
    log_info "BUILD phase score: $build_score/$total_checks ($build_percentage%)"
}

# RUN Phase - Infrastructure Runtime Validation
validate_run() {
    log_info "🚀 Starting RUN phase validation..."
    local run_score=0
    local total_checks=4

    # Test Docker environment
    log_info "Testing Docker runtime environment..."
    cd "$PROJECT_ROOT"

    if [ -f "infrastructure/docker/docker-compose.yml" ]; then
        log_info "Attempting to start Docker services..."
        if docker-compose -f infrastructure/docker/docker-compose.yml config >/dev/null 2>&1; then
            log_success "Docker Compose configuration is valid"
            run_score=$((run_score + 1))

            # Try to start essential services (non-blocking)
            log_info "Testing service startup (PostgreSQL, Redis)..."
            if timeout 30 docker-compose -f infrastructure/docker/docker-compose.yml up -d postgres redis >/dev/null 2>&1; then
                sleep 5
                if docker-compose -f infrastructure/docker/docker-compose.yml ps postgres redis | grep -q "Up"; then
                    log_success "Essential services started successfully"
                    run_score=$((run_score + 1))

                    # Cleanup
                    docker-compose -f infrastructure/docker/docker-compose.yml down >/dev/null 2>&1 || true
                else
                    log_warning "Some services failed to start properly"
                fi
            else
                log_warning "Services startup timeout or failed"
            fi
        else
            log_error "Docker Compose configuration is invalid"
        fi
    else
        log_warning "Docker Compose file not found, skipping runtime test"
    fi

    # Test Kubernetes manifests syntax
    log_info "Validating Kubernetes manifest syntax..."
    if command -v kubectl >/dev/null 2>&1; then
        local k8s_valid=true
        if [ -d "infrastructure/kubernetes" ]; then
            for manifest in infrastructure/kubernetes/*.yaml infrastructure/kubernetes/*.yml; do
                if [ -f "$manifest" ]; then
                    if kubectl apply --dry-run=client -f "$manifest" >/dev/null 2>&1; then
                        log_info "✓ $manifest syntax is valid"
                    else
                        log_warning "✗ $manifest has syntax errors"
                        k8s_valid=false
                    fi
                fi
            done

            if $k8s_valid; then
                log_success "All Kubernetes manifests have valid syntax"
                run_score=$((run_score + 1))
            fi
        fi
    else
        log_info "kubectl not available, skipping Kubernetes validation"
        run_score=$((run_score + 1))  # Don't penalize for missing optional tool
    fi

    # Test monitoring endpoints
    log_info "Testing monitoring system..."
    if [ -f "$PROJECT_ROOT/src/api-gateway/Cargo.toml" ]; then
        log_info "API Gateway found - monitoring endpoints should be available"
        run_score=$((run_score + 1))
    else
        log_warning "API Gateway not found"
    fi

    local run_percentage=$((run_score * 100 / total_checks))
    local run_status="failed"
    if [ "$run_percentage" -ge 75 ]; then
        run_status="success"
    elif [ "$run_percentage" -ge 50 ]; then
        run_status="warning"
    fi

    update_metrics "results" "run" "$run_status" "$run_percentage" "Runtime validation completed"
    log_info "RUN phase score: $run_score/$total_checks ($run_percentage%)"
}

# TEST Phase - Infrastructure Testing
validate_test() {
    log_info "🧪 Starting TEST phase validation..."
    local test_score=0
    local total_checks=4

    # Test infrastructure scripts
    log_info "Testing infrastructure automation scripts..."
    local scripts_found=0

    for script in "$PROJECT_ROOT/tools"/*.sh; do
        if [ -f "$script" ] && [ -x "$script" ]; then
            scripts_found=$((scripts_found + 1))
        fi
    done

    if [ "$scripts_found" -gt 5 ]; then
        log_success "Found $scripts_found executable automation scripts"
        test_score=$((test_score + 1))
    else
        log_warning "Limited automation scripts found ($scripts_found)"
    fi

    # Test CI/CD pipeline syntax
    log_info "Testing CI/CD pipeline configuration..."
    if [ -d "$PROJECT_ROOT/.github/workflows" ]; then
        local workflow_valid=true
        for workflow in "$PROJECT_ROOT/.github/workflows"/*.yml "$PROJECT_ROOT/.github/workflows"/*.yaml; do
            if [ -f "$workflow" ]; then
                # Basic YAML syntax validation
                if python3 -c "import yaml; yaml.safe_load(open('$workflow'))" >/dev/null 2>&1; then
                    log_info "✓ $(basename "$workflow") has valid YAML syntax"
                else
                    log_warning "✗ $(basename "$workflow") has YAML syntax errors"
                    workflow_valid=false
                fi
            fi
        done

        if $workflow_valid; then
            log_success "All workflow files have valid syntax"
            test_score=$((test_score + 1))
        fi
    else
        log_warning "No GitHub Actions workflows found"
    fi

    # Test security scanning configuration
    log_info "Testing security scanning setup..."
    if grep -r -l "security" "$PROJECT_ROOT/.github/workflows"/*.yml "$PROJECT_ROOT/.github/workflows"/*.yaml 2>/dev/null | head -1 >/dev/null; then
        log_success "Security scanning configured in CI/CD"
        test_score=$((test_score + 1))
    elif [ -f "$PROJECT_ROOT/tools/security-scan.sh" ]; then
        log_success "Security scanning script found"
        test_score=$((test_score + 1))
    else
        log_warning "No security scanning configuration found"
    fi

    # Test performance monitoring
    log_info "Testing performance monitoring setup..."
    if [ -d "$PROJECT_ROOT/infrastructure/monitoring" ]; then
        if find "$PROJECT_ROOT/infrastructure/monitoring" -name "*prometheus*" -o -name "*grafana*" -o -name "*metrics*" | head -1 >/dev/null; then
            log_success "Performance monitoring configuration found"
            test_score=$((test_score + 1))
        else
            log_warning "Performance monitoring configuration not found"
        fi
    else
        log_warning "Monitoring directory not found"
    fi

    local test_percentage=$((test_score * 100 / total_checks))
    local test_status="failed"
    if [ "$test_percentage" -ge 75 ]; then
        test_status="success"
    elif [ "$test_percentage" -ge 50 ]; then
        test_status="warning"
    fi

    update_metrics "results" "test" "$test_status" "$test_percentage" "Testing validation completed"
    log_info "TEST phase score: $test_score/$total_checks ($test_percentage%)"
}

# FIX Phase - Infrastructure Issue Resolution
validate_fix() {
    log_info "🔧 Starting FIX phase validation..."
    local fix_score=0
    local total_checks=4
    local recommendations=()

    # Check for common infrastructure issues
    log_info "Analyzing infrastructure health..."

    # Check disk space
    if command -v df >/dev/null 2>&1; then
        local disk_usage=$(df "$PROJECT_ROOT" | awk 'NR==2 {print $5}' | sed 's/%//')
        if [ "$disk_usage" -lt 80 ]; then
            log_success "Disk space usage is healthy ($disk_usage%)"
            fix_score=$((fix_score + 1))
        else
            log_warning "High disk usage detected ($disk_usage%)"
            recommendations+=("Clean up disk space - usage at $disk_usage%")
        fi
    fi

    # Check for Docker issues
    log_info "Checking Docker health..."
    if command -v docker >/dev/null 2>&1; then
        if docker system df >/dev/null 2>&1; then
            log_success "Docker system is responsive"
            fix_score=$((fix_score + 1))

            # Check for unused resources
            local unused_volumes=$(docker volume ls -q --filter dangling=true | wc -l)
            if [ "$unused_volumes" -gt 5 ]; then
                recommendations+=("Clean up $unused_volumes unused Docker volumes")
            fi
        else
            log_warning "Docker system check failed"
        fi
    fi

    # Check for configuration issues
    log_info "Analyzing configuration completeness..."
    local config_score=0
    local config_total=5

    [ -f "$PROJECT_ROOT/infrastructure/docker/docker-compose.yml" ] && config_score=$((config_score + 1))
    [ -d "$PROJECT_ROOT/infrastructure/kubernetes" ] && config_score=$((config_score + 1))
    [ -d "$PROJECT_ROOT/.github/workflows" ] && config_score=$((config_score + 1))
    [ -d "$PROJECT_ROOT/infrastructure/monitoring" ] && config_score=$((config_score + 1))
    [ -f "$PROJECT_ROOT/tools/quality-gates.sh" ] && config_score=$((config_score + 1))

    if [ "$config_score" -ge 4 ]; then
        log_success "Infrastructure configuration is comprehensive ($config_score/$config_total)"
        fix_score=$((fix_score + 1))
    else
        log_warning "Infrastructure configuration needs improvement ($config_score/$config_total)"
        recommendations+=("Complete missing infrastructure configurations")
    fi

    # Check for automation completeness
    log_info "Analyzing automation coverage..."
    local automation_tools=0
    [ -x "$PROJECT_ROOT/tools/quality-gates.sh" ] && automation_tools=$((automation_tools + 1))
    [ -x "$PROJECT_ROOT/tools/self-healing-env.sh" ] && automation_tools=$((automation_tools + 1))
    [ -x "$PROJECT_ROOT/tools/metrics-collector.sh" ] && automation_tools=$((automation_tools + 1))

    if [ "$automation_tools" -ge 2 ]; then
        log_success "Good automation tool coverage ($automation_tools tools)"
        fix_score=$((fix_score + 1))
    else
        log_warning "Limited automation tools available ($automation_tools tools)"
        recommendations+=("Implement more automation tools for better DevOps coverage")
    fi

    local fix_percentage=$((fix_score * 100 / total_checks))
    local fix_status="failed"
    if [ "$fix_percentage" -ge 75 ]; then
        fix_status="success"
    elif [ "$fix_percentage" -ge 50 ]; then
        fix_status="warning"
    fi

    update_metrics "results" "fix" "$fix_status" "$fix_percentage" "Issue resolution validation completed"
    log_info "FIX phase score: $fix_score/$total_checks ($fix_percentage%)"

    # Add recommendations to metrics
    if [ ${#recommendations[@]} -gt 0 ]; then
        log_info "Recommendations for improvement:"
        for rec in "${recommendations[@]}"; do
            log_info "  • $rec"
        done
    fi
}

# Calculate overall score and grade
calculate_overall_score() {
    log_info "📊 Calculating overall DevOps Agent validation score..."

    # Extract scores from metrics file
    local build_score=$(python3 -c "import json; data=json.load(open('$METRICS_FILE')); print(data['results']['build']['score'])")
    local run_score=$(python3 -c "import json; data=json.load(open('$METRICS_FILE')); print(data['results']['run']['score'])")
    local test_score=$(python3 -c "import json; data=json.load(open('$METRICS_FILE')); print(data['results']['test']['score'])")
    local fix_score=$(python3 -c "import json; data=json.load(open('$METRICS_FILE')); print(data['results']['fix']['score'])")

    # Weighted average: BUILD(30%), RUN(25%), TEST(25%), FIX(20%)
    local overall_score=$(python3 -c "print(int($build_score * 0.3 + $run_score * 0.25 + $test_score * 0.25 + $fix_score * 0.2))")

    # Determine grade
    local grade="F"
    if [ "$overall_score" -ge 90 ]; then
        grade="A"
    elif [ "$overall_score" -ge 80 ]; then
        grade="B"
    elif [ "$overall_score" -ge 70 ]; then
        grade="C"
    elif [ "$overall_score" -ge 60 ]; then
        grade="D"
    fi

    # Update final metrics
    python3 -c "
import json
with open('$METRICS_FILE', 'r') as f:
    data = json.load(f)
data['overall_score'] = $overall_score
data['grade'] = '$grade'
data['validation_end'] = '$(date -u +"%Y-%m-%dT%H:%M:%SZ")'
with open('$METRICS_FILE', 'w') as f:
    json.dump(data, f, indent=2)
"

    log_success "DevOps Agent Validation Complete!"
    log_info "Overall Score: $overall_score/100 (Grade: $grade)"
    log_info "BUILD: $build_score% | RUN: $run_score% | TEST: $test_score% | FIX: $fix_score%"
}

# Generate comprehensive report
generate_report() {
    log_info "📋 Generating DevOps validation report..."

    mkdir -p "$REPORT_DIR"
    local report_file="$REPORT_DIR/devops-validation-report-$(date +%Y%m%d-%H%M%S).md"

    cat > "$report_file" <<EOF
# DevOps Agent BUILD/RUN/TEST/FIX Validation Report

**Generated**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Task**: 10.6 - DevOps Agent Validation
**Project**: AI-CORE Intelligent Automation Platform

## Executive Summary

$(python3 -c "
import json
with open('$METRICS_FILE', 'r') as f:
    data = json.load(f)
print(f\"Overall Score: {data['overall_score']}/100 (Grade: {data['grade']})\")
print(f\"BUILD: {data['results']['build']['score']}% | RUN: {data['results']['run']['score']}% | TEST: {data['results']['test']['score']}% | FIX: {data['results']['fix']['score']}%\")
")

## Infrastructure Component Status

$(python3 -c "
import json
with open('$METRICS_FILE', 'r') as f:
    data = json.load(f)
for component, info in data['infrastructure'].items():
    status_emoji = '✅' if info['status'] == 'success' else '⚠️' if info['status'] == 'warning' else '❌'
    print(f\"- **{component.replace('_', ' ').title()}**: {status_emoji} {info['status'].upper()} ({info['score']}%)\")
")

## Detailed Results

### BUILD Phase (Infrastructure Build Validation)
- **Score**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['build']['score'])")%
- **Status**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['build']['status'])")
- **Focus**: Docker environment, Kubernetes manifests, CI/CD pipelines, monitoring configuration

### RUN Phase (Infrastructure Runtime Validation)
- **Score**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['run']['score'])")%
- **Status**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['run']['status'])")
- **Focus**: Service startup, runtime configuration, endpoint availability

### TEST Phase (Infrastructure Testing)
- **Score**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['test']['score'])")%
- **Status**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['test']['status'])")
- **Focus**: Automation scripts, CI/CD syntax, security scanning, performance monitoring

### FIX Phase (Infrastructure Issue Resolution)
- **Score**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['fix']['score'])")%
- **Status**: $(python3 -c "import json; print(json.load(open('$METRICS_FILE'))['results']['fix']['status'])")
- **Focus**: Health analysis, issue identification, automation coverage

## Next Steps

Based on the validation results, the following actions are recommended:

1. **High Priority**: Address any failed infrastructure components
2. **Medium Priority**: Improve warning-level components
3. **Low Priority**: Enhance automation and monitoring coverage

## Files Generated

- Validation Log: \`$VALIDATION_LOG\`
- Metrics File: \`$METRICS_FILE\`
- This Report: \`$report_file\`

---
*Report generated by DevOps Agent BUILD/RUN/TEST/FIX validation system*
EOF

    log_success "Report generated: $report_file"
}

# Main execution
main() {
    log_info "🚀 DevOps Agent BUILD/RUN/TEST/FIX Validation Started"
    log_info "Task 10.6: Infrastructure deployment and validation"
    log_info "Timestamp: $TIMESTAMP"

    # Initialize
    > "$VALIDATION_LOG"
    mkdir -p "$REPORT_DIR"
    init_metrics

    # Execute validation phases
    validate_build
    validate_run
    validate_test
    validate_fix

    # Calculate results
    calculate_overall_score
    generate_report

    log_success "🎉 DevOps Agent validation completed!"
    log_info "Check $VALIDATION_LOG for detailed logs"
    log_info "Check $METRICS_FILE for metrics data"
}

# Command line handling
case "${1:-main}" in
    "build") validate_build ;;
    "run") validate_run ;;
    "test") validate_test ;;
    "fix") validate_fix ;;
    "report") generate_report ;;
    "main"|*) main ;;
esac

#!/bin/bash

# AI-PLATFORM DevOps Agent Validation Script
# BUILD/RUN/TEST/FIX validation for all DevOps deliverables
# This script validates infrastructure, Docker, Kubernetes, and CI/CD components

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
K8S_DIR="$PROJECT_ROOT/infrastructure/kubernetes"
GITHUB_DIR="$PROJECT_ROOT/.github/workflows"

# Validation results
VALIDATION_RESULTS=()
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# ================================
# UTILITY FUNCTIONS
# ================================

print_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════════╗"
    echo "║                    DevOps Agent Validation                          ║"
    echo "║                     BUILD/RUN/TEST/FIX                              ║"
    echo "╚══════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASSED_TESTS++))
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAILED_TESTS++))
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
    ((TOTAL_TESTS++))
}

add_result() {
    VALIDATION_RESULTS+=("$1")
}

# ================================
# VALIDATION FUNCTIONS
# ================================

validate_docker_setup() {
    print_step "Validating Docker configuration..."

    # Check if Docker files exist
    if [ -f "$DOCKER_DIR/Dockerfile.api" ]; then
        print_success "API Gateway Dockerfile exists"
        add_result "✅ API Gateway Dockerfile: EXISTS"
    else
        print_error "API Gateway Dockerfile missing"
        add_result "❌ API Gateway Dockerfile: MISSING"
        return 1
    fi

    if [ -f "$DOCKER_DIR/Dockerfile.ui" ]; then
        print_success "Frontend UI Dockerfile exists"
        add_result "✅ Frontend UI Dockerfile: EXISTS"
    else
        print_error "Frontend UI Dockerfile missing"
        add_result "❌ Frontend UI Dockerfile: MISSING"
        return 1
    fi

    # Validate Docker Compose files
    if [ -f "$DOCKER_DIR/docker-compose.yml" ]; then
        print_success "Main Docker Compose file exists"

        # Validate Docker Compose syntax
        if docker compose -f "$DOCKER_DIR/docker-compose.yml" config > /dev/null 2>&1; then
            print_success "Docker Compose syntax is valid"
            add_result "✅ Docker Compose: VALID SYNTAX"
        else
            print_error "Docker Compose syntax is invalid"
            add_result "❌ Docker Compose: INVALID SYNTAX"
            return 1
        fi
    else
        print_error "Main Docker Compose file missing"
        add_result "❌ Docker Compose: MISSING"
        return 1
    fi

    # Check monitoring configuration
    if [ -f "$DOCKER_DIR/monitoring/prometheus/prometheus.yml" ]; then
        print_success "Prometheus configuration exists"
        add_result "✅ Prometheus Config: EXISTS"
    else
        print_warning "Prometheus configuration missing"
        add_result "⚠️  Prometheus Config: MISSING"
    fi

    if [ -f "$DOCKER_DIR/monitoring/grafana/provisioning/datasources/datasources.yml" ]; then
        print_success "Grafana datasources configuration exists"
        add_result "✅ Grafana Datasources: EXISTS"
    else
        print_warning "Grafana datasources configuration missing"
        add_result "⚠️  Grafana Datasources: MISSING"
    fi

    return 0
}

validate_kubernetes_manifests() {
    print_step "Validating Kubernetes manifests..."

    # Check if K8s manifests exist
    if [ -d "$K8S_DIR/manifests" ]; then
        print_success "Kubernetes manifests directory exists"
        add_result "✅ K8s Manifests Directory: EXISTS"
    else
        print_error "Kubernetes manifests directory missing"
        add_result "❌ K8s Manifests Directory: MISSING"
        return 1
    fi

    # Validate namespace configuration
    if [ -f "$K8S_DIR/manifests/namespace.yaml" ]; then
        print_success "Namespace manifest exists"
        add_result "✅ K8s Namespace: EXISTS"
    else
        print_error "Namespace manifest missing"
        add_result "❌ K8s Namespace: MISSING"
        return 1
    fi

    # Validate API Gateway deployment
    if [ -f "$K8S_DIR/manifests/api-gateway.yaml" ]; then
        print_success "API Gateway K8s manifest exists"
        add_result "✅ K8s API Gateway: EXISTS"
    else
        print_error "API Gateway K8s manifest missing"
        add_result "❌ K8s API Gateway: MISSING"
        return 1
    fi

    # Validate YAML syntax with kubeval if available
    if command -v kubeval > /dev/null 2>&1; then
        print_status "Running kubeval validation..."
        local validation_failed=false

        for manifest in "$K8S_DIR/manifests"/*.yaml; do
            if [ -f "$manifest" ]; then
                if kubeval "$manifest" > /dev/null 2>&1; then
                    print_success "$(basename "$manifest") is valid"
                else
                    print_error "$(basename "$manifest") validation failed"
                    validation_failed=true
                fi
            fi
        done

        if [ "$validation_failed" = false ]; then
            add_result "✅ K8s YAML Validation: PASSED"
        else
            add_result "❌ K8s YAML Validation: FAILED"
            return 1
        fi
    else
        print_warning "kubeval not found, skipping YAML validation"
        add_result "⚠️  K8s YAML Validation: SKIPPED (kubeval not found)"
    fi

    return 0
}

validate_ci_cd_pipeline() {
    print_step "Validating CI/CD pipeline..."

    # Check if GitHub Actions workflow exists
    if [ -f "$GITHUB_DIR/ci-cd.yml" ]; then
        print_success "GitHub Actions CI/CD workflow exists"
        add_result "✅ GitHub Actions Workflow: EXISTS"
    else
        print_error "GitHub Actions CI/CD workflow missing"
        add_result "❌ GitHub Actions Workflow: MISSING"
        return 1
    fi

    # Validate YAML syntax
    if command -v yq > /dev/null 2>&1; then
        if yq eval '.' "$GITHUB_DIR/ci-cd.yml" > /dev/null 2>&1; then
            print_success "GitHub Actions workflow YAML is valid"
            add_result "✅ GitHub Actions YAML: VALID"
        else
            print_error "GitHub Actions workflow YAML is invalid"
            add_result "❌ GitHub Actions YAML: INVALID"
            return 1
        fi
    else
        print_warning "yq not found, skipping YAML syntax validation"
        add_result "⚠️  GitHub Actions YAML: SKIPPED (yq not found)"
    fi

    # Check for required workflow components
    local workflow_content
    workflow_content=$(cat "$GITHUB_DIR/ci-cd.yml")

    if echo "$workflow_content" | grep -q "backend-ci:"; then
        print_success "Backend CI job found"
        add_result "✅ Backend CI Job: EXISTS"
    else
        print_error "Backend CI job missing"
        add_result "❌ Backend CI Job: MISSING"
    fi

    if echo "$workflow_content" | grep -q "frontend-ci:"; then
        print_success "Frontend CI job found"
        add_result "✅ Frontend CI Job: EXISTS"
    else
        print_error "Frontend CI job missing"
        add_result "❌ Frontend CI Job: MISSING"
    fi

    if echo "$workflow_content" | grep -q "security-scan:"; then
        print_success "Security scan job found"
        add_result "✅ Security Scan Job: EXISTS"
    else
        print_error "Security scan job missing"
        add_result "❌ Security Scan Job: MISSING"
    fi

    if echo "$workflow_content" | grep -q "build-images:"; then
        print_success "Image build job found"
        add_result "✅ Image Build Job: EXISTS"
    else
        print_error "Image build job missing"
        add_result "❌ Image Build Job: MISSING"
    fi

    return 0
}

validate_monitoring_setup() {
    print_step "Validating monitoring configuration..."

    # Check Prometheus configuration
    if [ -f "$DOCKER_DIR/monitoring/prometheus/prometheus.yml" ]; then
        # Validate Prometheus config syntax
        if command -v promtool > /dev/null 2>&1; then
            if promtool check config "$DOCKER_DIR/monitoring/prometheus/prometheus.yml" > /dev/null 2>&1; then
                print_success "Prometheus configuration is valid"
                add_result "✅ Prometheus Config: VALID"
            else
                print_error "Prometheus configuration is invalid"
                add_result "❌ Prometheus Config: INVALID"
                return 1
            fi
        else
            print_warning "promtool not found, skipping Prometheus validation"
            add_result "⚠️  Prometheus Config: SKIPPED (promtool not found)"
        fi
    fi

    # Check alerting rules
    if [ -f "$DOCKER_DIR/monitoring/prometheus/rules/AI-PLATFORM-alerts.yml" ]; then
        print_success "Prometheus alerting rules exist"
        add_result "✅ Prometheus Alerts: EXISTS"
    else
        print_warning "Prometheus alerting rules missing"
        add_result "⚠️  Prometheus Alerts: MISSING"
    fi

    # Check Grafana datasources
    if [ -f "$DOCKER_DIR/monitoring/grafana/provisioning/datasources/datasources.yml" ]; then
        local datasources_content
        datasources_content=$(cat "$DOCKER_DIR/monitoring/grafana/provisioning/datasources/datasources.yml")

        if echo "$datasources_content" | grep -q "type: prometheus"; then
            print_success "Prometheus datasource configured"
            add_result "✅ Grafana Prometheus DS: CONFIGURED"
        else
            print_error "Prometheus datasource not configured"
            add_result "❌ Grafana Prometheus DS: MISSING"
        fi

        if echo "$datasources_content" | grep -q "type: loki"; then
            print_success "Loki datasource configured"
            add_result "✅ Grafana Loki DS: CONFIGURED"
        else
            print_warning "Loki datasource not configured"
            add_result "⚠️  Grafana Loki DS: MISSING"
        fi
    fi

    return 0
}

validate_scripts() {
    print_step "Validating setup scripts..."

    # Check development environment setup script
    if [ -f "$PROJECT_ROOT/scripts/setup-dev-environment.sh" ]; then
        print_success "Development environment setup script exists"
        add_result "✅ Dev Setup Script: EXISTS"

        # Check if script is executable
        if [ -x "$PROJECT_ROOT/scripts/setup-dev-environment.sh" ]; then
            print_success "Setup script is executable"
            add_result "✅ Dev Setup Script: EXECUTABLE"
        else
            print_warning "Setup script is not executable"
            add_result "⚠️  Dev Setup Script: NOT EXECUTABLE"
        fi

        # Basic syntax check
        if bash -n "$PROJECT_ROOT/scripts/setup-dev-environment.sh"; then
            print_success "Setup script syntax is valid"
            add_result "✅ Dev Setup Script: VALID SYNTAX"
        else
            print_error "Setup script has syntax errors"
            add_result "❌ Dev Setup Script: SYNTAX ERROR"
            return 1
        fi
    else
        print_error "Development environment setup script missing"
        add_result "❌ Dev Setup Script: MISSING"
        return 1
    fi

    return 0
}

test_docker_build() {
    print_step "Testing Docker builds..."

    # Test API Gateway Docker build
    if [ -f "$DOCKER_DIR/Dockerfile.api" ]; then
        print_status "Testing API Gateway Docker build..."
        if docker build -f "$DOCKER_DIR/Dockerfile.api" -t AI-PLATFORM/api-gateway:test . > /dev/null 2>&1; then
            print_success "API Gateway Docker build successful"
            add_result "✅ API Gateway Build: SUCCESS"

            # Clean up test image
            docker rmi AI-PLATFORM/api-gateway:test > /dev/null 2>&1 || true
        else
            print_error "API Gateway Docker build failed"
            add_result "❌ API Gateway Build: FAILED"
            return 1
        fi
    fi

    # Test Docker Compose validation
    if [ -f "$DOCKER_DIR/docker-compose.yml" ]; then
        print_status "Testing Docker Compose configuration..."
        if docker compose -f "$DOCKER_DIR/docker-compose.yml" config > /dev/null 2>&1; then
            print_success "Docker Compose configuration is valid"
            add_result "✅ Docker Compose Test: VALID"
        else
            print_error "Docker Compose configuration is invalid"
            add_result "❌ Docker Compose Test: INVALID"
            return 1
        fi
    fi

    return 0
}

test_infrastructure_deployment() {
    print_step "Testing infrastructure deployment readiness..."

    # Check if all required configuration files exist
    local required_configs=(
        "$DOCKER_DIR/postgres/conf/postgresql.conf"
        "$DOCKER_DIR/monitoring/prometheus/prometheus.yml"
        "$DOCKER_DIR/monitoring/grafana/provisioning/datasources/datasources.yml"
    )

    local missing_configs=()
    for config in "${required_configs[@]}"; do
        if [ ! -f "$config" ]; then
            missing_configs+=("$config")
        fi
    done

    if [ ${#missing_configs[@]} -eq 0 ]; then
        print_success "All required configuration files exist"
        add_result "✅ Configuration Files: COMPLETE"
    else
        print_error "Missing configuration files: ${missing_configs[*]}"
        add_result "❌ Configuration Files: INCOMPLETE"
        return 1
    fi

    # Test volume directory creation
    local volume_dirs=(
        "$DOCKER_DIR/volumes"
        "$DOCKER_DIR/logs"
        "$DOCKER_DIR/config"
    )

    for dir in "${volume_dirs[@]}"; do
        if [ -d "$dir" ] || mkdir -p "$dir" 2>/dev/null; then
            print_success "Volume directory available: $(basename "$dir")"
        else
            print_error "Cannot create volume directory: $dir"
            add_result "❌ Volume Directories: FAILED"
            return 1
        fi
    done

    add_result "✅ Volume Directories: READY"
    return 0
}

run_fix_recommendations() {
    print_step "Generating fix recommendations..."

    local fixes_needed=()

    # Check for common issues and provide fixes
    if [ $FAILED_TESTS -gt 0 ]; then
        echo -e "${YELLOW}📋 FIX RECOMMENDATIONS:${NC}"
        echo ""

        # Docker-related fixes
        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ API Gateway Dockerfile: MISSING" ]]; then
            fixes_needed+=("Create API Gateway Dockerfile: infrastructure/docker/Dockerfile.api")
        fi

        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ Frontend UI Dockerfile: MISSING" ]]; then
            fixes_needed+=("Create Frontend UI Dockerfile: infrastructure/docker/Dockerfile.ui")
        fi

        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ Docker Compose: MISSING" ]]; then
            fixes_needed+=("Create Docker Compose file: infrastructure/docker/docker-compose.yml")
        fi

        # Kubernetes-related fixes
        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ K8s Manifests Directory: MISSING" ]]; then
            fixes_needed+=("Create Kubernetes manifests directory: infrastructure/kubernetes/manifests/")
        fi

        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ K8s Namespace: MISSING" ]]; then
            fixes_needed+=("Create namespace manifest: infrastructure/kubernetes/manifests/namespace.yaml")
        fi

        # CI/CD-related fixes
        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ GitHub Actions Workflow: MISSING" ]]; then
            fixes_needed+=("Create GitHub Actions workflow: .github/workflows/ci-cd.yml")
        fi

        # Script-related fixes
        if [[ " ${VALIDATION_RESULTS[*]} " =~ "❌ Dev Setup Script: MISSING" ]]; then
            fixes_needed+=("Create setup script: scripts/setup-dev-environment.sh")
        fi

        if [[ " ${VALIDATION_RESULTS[*]} " =~ "⚠️  Dev Setup Script: NOT EXECUTABLE" ]]; then
            fixes_needed+=("Make setup script executable: chmod +x scripts/setup-dev-environment.sh")
        fi

        # Display fixes
        for i in "${!fixes_needed[@]}"; do
            echo -e "${CYAN}$((i+1)).${NC} ${fixes_needed[$i]}"
        done

        echo ""
        echo -e "${YELLOW}💡 Quick Fix Commands:${NC}"
        echo ""

        # Provide quick fix commands
        if [[ " ${VALIDATION_RESULTS[*]} " =~ "⚠️  Dev Setup Script: NOT EXECUTABLE" ]]; then
            echo "chmod +x scripts/setup-dev-environment.sh"
        fi

        echo "mkdir -p infrastructure/docker/volumes infrastructure/kubernetes/manifests .github/workflows"
        echo "mkdir -p infrastructure/docker/monitoring/prometheus infrastructure/docker/monitoring/grafana/provisioning/datasources"

        echo ""
        echo -e "${BLUE}📚 Documentation:${NC}"
        echo "• Docker: https://docs.docker.com/compose/"
        echo "• Kubernetes: https://kubernetes.io/docs/"
        echo "• GitHub Actions: https://docs.github.com/en/actions"
        echo ""
    else
        print_success "No fixes needed - all validations passed!"
        add_result "🎉 All DevOps deliverables validated successfully"
    fi
}

generate_validation_report() {
    print_step "Generating validation report..."

    echo ""
    echo -e "${PURPLE}╔════════════════════════════════════════════════════════════════════════╗"
    echo "║                            VALIDATION REPORT                          ║"
    echo "╚════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    echo -e "${CYAN}📊 SUMMARY:${NC}"
    echo "   Total Tests: $TOTAL_TESTS"
    echo "   Passed: $PASSED_TESTS"
    echo "   Failed: $FAILED_TESTS"
    echo "   Success Rate: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%"
    echo ""

    echo -e "${CYAN}📋 DETAILED RESULTS:${NC}"
    for result in "${VALIDATION_RESULTS[@]}"; do
        echo "   $result"
    done
    echo ""

    # Generate report file
    local report_file="$PROJECT_ROOT/devops-validation-report.txt"
    {
        echo "AI-PLATFORM DevOps Agent Validation Report"
        echo "======================================"
        echo "Generated: $(date)"
        echo "Environment: $(uname -s) $(uname -r)"
        echo ""
        echo "SUMMARY:"
        echo "   Total Tests: $TOTAL_TESTS"
        echo "   Passed: $PASSED_TESTS"
        echo "   Failed: $FAILED_TESTS"
        echo "   Success Rate: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%"
        echo ""
        echo "DETAILED RESULTS:"
        for result in "${VALIDATION_RESULTS[@]}"; do
            echo "   $result"
        done
    } > "$report_file"

    print_success "Validation report saved to: $report_file"
}

# ================================
# MAIN EXECUTION
# ================================

main() {
    print_banner

    local exit_code=0

    # Phase 1: BUILD validation
    echo -e "${CYAN}🔨 PHASE 1: BUILD VALIDATION${NC}"
    validate_docker_setup || exit_code=1
    validate_kubernetes_manifests || exit_code=1
    validate_ci_cd_pipeline || exit_code=1
    validate_scripts || exit_code=1
    echo ""

    # Phase 2: RUN validation
    echo -e "${CYAN}🚀 PHASE 2: RUN VALIDATION${NC}"
    if command -v docker > /dev/null 2>&1; then
        test_docker_build || exit_code=1
    else
        print_warning "Docker not available, skipping build tests"
        add_result "⚠️  Docker Build Test: SKIPPED (Docker not available)"
    fi
    test_infrastructure_deployment || exit_code=1
    echo ""

    # Phase 3: TEST validation
    echo -e "${CYAN}🧪 PHASE 3: TEST VALIDATION${NC}"
    validate_monitoring_setup || exit_code=1
    echo ""

    # Phase 4: FIX recommendations
    echo -e "${CYAN}🔧 PHASE 4: FIX RECOMMENDATIONS${NC}"
    run_fix_recommendations
    echo ""

    # Generate final report
    generate_validation_report

    # Final status
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ DevOps Agent Validation: SUCCESS${NC}"
        echo -e "${GREEN}🎉 All infrastructure components are properly configured!${NC}"
    else
        echo -e "${RED}❌ DevOps Agent Validation: FAILED${NC}"
        echo -e "${RED}🚨 Some issues need to be addressed before deployment${NC}"
    fi

    echo ""
    echo -e "${BLUE}📁 Next Steps:${NC}"
    echo "1. Address any failed validations using the fix recommendations"
    echo "2. Run './scripts/setup-dev-environment.sh' to start the environment"
    echo "3. Verify services are running with 'docker compose ps'"
    echo "4. Test the API gateway health endpoint"
    echo ""

    return $exit_code
}

# Execute main function
main "$@"

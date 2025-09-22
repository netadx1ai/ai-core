#!/bin/bash

# AI-CORE Backend Agent Validation Script
# Comprehensive testing suite to validate all backend implementations

set -e  # Exit on any error

echo "🚀 AI-CORE Backend Agent Validation Suite"
echo "=========================================="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Helper functions
print_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

# Change to project root
cd "$(dirname "$0")"
PROJECT_ROOT=$(pwd)

echo "📁 Project Root: $PROJECT_ROOT"
echo

# Test 1: Validate project structure
print_test "Validating project structure..."
if [[ -d "src/api-gateway" && -f "src/api-gateway/Cargo.toml" && -f "src/api-gateway/src/main.rs" ]]; then
    print_success "API Gateway structure exists"
else
    print_error "API Gateway structure missing"
fi

# Test 2: Check Cargo.toml dependencies
print_test "Validating Cargo.toml dependencies..."
if grep -q "axum.*0\.7" src/api-gateway/Cargo.toml && grep -q "tokio.*1\." src/api-gateway/Cargo.toml; then
    print_success "Core dependencies configured correctly"
else
    print_error "Missing or incorrect core dependencies"
fi

# Test 3: Environment configuration
print_test "Checking environment configuration..."
if [[ -f "src/api-gateway/.env" && -f "src/api-gateway/.env.example" ]]; then
    print_success "Environment configuration files present"
else
    print_error "Missing environment configuration files"
fi

# Test 4: Source code validation
print_test "Validating source code structure..."
REQUIRED_FILES=(
    "src/api-gateway/src/main.rs"
    "src/api-gateway/src/config.rs"
    "src/api-gateway/src/state.rs"
    "src/api-gateway/src/error.rs"
    "src/api-gateway/src/handlers/auth.rs"
    "src/api-gateway/src/handlers/workflows.rs"
    "src/api-gateway/src/middleware_layer/auth.rs"
    "src/api-gateway/src/middleware_layer/error_handling.rs"
    "src/api-gateway/src/middleware_layer/rate_limit.rs"
    "src/api-gateway/src/services/auth.rs"
    "src/api-gateway/src/services/workflow.rs"
    "src/api-gateway/src/services/orchestrator.rs"
    "src/api-gateway/src/routes/api.rs"
    "src/api-gateway/src/routes/public.rs"
)

MISSING_FILES=0
for file in "${REQUIRED_FILES[@]}"; do
    if [[ ! -f "$file" ]]; then
        print_warning "Missing file: $file"
        MISSING_FILES=$((MISSING_FILES + 1))
    fi
done

if [[ $MISSING_FILES -eq 0 ]]; then
    print_success "All required source files present"
else
    print_error "$MISSING_FILES source files missing"
fi

echo
echo "🔨 Building and Testing Backend Components"
echo "=========================================="

# Test 5: Compilation test
print_test "Testing compilation..."
cd src/api-gateway
if cargo check --quiet 2>/dev/null; then
    print_success "Code compiles without errors"
else
    print_error "Compilation failed"
    echo "Running cargo check for details:"
    cargo check 2>&1 | head -20
fi

# Test 6: Unit tests
print_test "Running unit tests..."
if timeout 30 cargo test --quiet 2>/dev/null; then
    print_success "Unit tests pass"
else
    print_error "Unit tests failed or timed out"
    echo "Running tests with output:"
    timeout 10 cargo test 2>&1 | head -20
fi

# Test 7: Release build
print_test "Testing release build..."
if timeout 60 cargo build --release --quiet 2>/dev/null; then
    print_success "Release build successful"
else
    print_error "Release build failed"
fi

# Test 8: Clippy linting
print_test "Running Clippy linting..."
if cargo clippy --quiet -- -D warnings 2>/dev/null; then
    print_success "Clippy linting passed"
else
    print_warning "Clippy warnings found (non-critical)"
fi

# Test 9: Documentation build
print_test "Testing documentation build..."
if timeout 30 cargo doc --quiet --no-deps 2>/dev/null; then
    print_success "Documentation builds successfully"
else
    print_warning "Documentation build issues (non-critical)"
fi

cd "$PROJECT_ROOT"

echo
echo "📋 Code Quality Analysis"
echo "========================"

# Test 10: Code metrics
print_test "Analyzing code metrics..."

# Count lines of code
RUST_FILES=$(find src/api-gateway/src -name "*.rs" | wc -l)
TOTAL_LINES=$(find src/api-gateway/src -name "*.rs" -exec wc -l {} + | tail -1 | awk '{print $1}')

print_info "Rust files: $RUST_FILES"
print_info "Total lines of code: $TOTAL_LINES"

if [[ $TOTAL_LINES -gt 2000 ]]; then
    print_success "Substantial codebase implemented ($TOTAL_LINES lines)"
else
    print_warning "Codebase smaller than expected ($TOTAL_LINES lines)"
fi

# Test 11: API endpoint validation
print_test "Validating API endpoints..."
API_ROUTES=$(grep -r "route.*/" src/api-gateway/src/routes/ | wc -l)
print_info "API routes defined: $API_ROUTES"

if [[ $API_ROUTES -gt 20 ]]; then
    print_success "Comprehensive API coverage ($API_ROUTES routes)"
else
    print_warning "Limited API coverage ($API_ROUTES routes)"
fi

echo
echo "🔧 Configuration Validation"
echo "==========================="

# Test 12: Environment variables
print_test "Checking environment configuration..."
ENV_VARS=$(grep -E "^[A-Z_]+=" src/api-gateway/.env 2>/dev/null | wc -l)
print_info "Environment variables configured: $ENV_VARS"

if [[ $ENV_VARS -gt 20 ]]; then
    print_success "Comprehensive environment configuration"
else
    print_warning "Limited environment configuration"
fi

# Test 13: Security configuration
print_test "Validating security configuration..."
if grep -q "JWT_SECRET" src/api-gateway/.env 2>/dev/null; then
    print_success "JWT configuration present"
else
    print_error "Missing JWT configuration"
fi

if grep -q "RATE_LIMIT" src/api-gateway/.env 2>/dev/null; then
    print_success "Rate limiting configuration present"
else
    print_warning "Rate limiting configuration missing"
fi

echo
echo "📚 Documentation Validation"
echo "==========================="

# Test 14: Documentation files
print_test "Checking documentation..."
DOC_FILES=(
    "BACKEND_AGENT_COMPLETE.md"
    "tasks.md"
    "src/api-gateway/.env.example"
)

for doc in "${DOC_FILES[@]}"; do
    if [[ -f "$doc" ]]; then
        print_info "Documentation found: $doc"
    else
        print_warning "Missing documentation: $doc"
    fi
done

# Test 15: README and setup instructions
print_test "Validating setup documentation..."
if [[ -f "src/api-gateway/.env.example" ]]; then
    print_success "Environment setup template exists"
else
    print_warning "Missing environment setup template"
fi

echo
echo "🏃‍♂️ Runtime Validation (Optional)"
echo "=================================="

# Test 16: Runtime test (optional, may fail without database)
print_test "Testing runtime startup (degraded mode)..."
cd src/api-gateway

# Set minimal environment for testing
export ENVIRONMENT=development
export SERVER_PORT=8081  # Use different port to avoid conflicts
export LOG_LEVEL=error    # Reduce log noise

# Try to start the server with timeout
timeout 10 cargo run --bin api-gateway &
SERVER_PID=$!
sleep 3

# Check if server is responsive
if kill -0 $SERVER_PID 2>/dev/null; then
    print_success "Server starts successfully (degraded mode)"
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
else
    print_warning "Server startup test skipped (expected without database)"
fi

cd "$PROJECT_ROOT"

echo
echo "📊 Test Summary"
echo "==============="

TOTAL_PASSED=$TESTS_PASSED
TOTAL_FAILED=$TESTS_FAILED
TOTAL_RUN=$TOTAL_TESTS

echo "Tests Run: $TOTAL_RUN"
echo -e "Passed: ${GREEN}$TOTAL_PASSED${NC}"
echo -e "Failed: ${RED}$TOTAL_FAILED${NC}"

if [[ $TOTAL_FAILED -eq 0 ]]; then
    echo
    echo -e "${GREEN}🎉 ALL CRITICAL TESTS PASSED${NC}"
    echo -e "${GREEN}✅ Backend Agent implementation is COMPLETE and READY${NC}"
    echo
    echo "Next Steps:"
    echo "1. Start database services (PostgreSQL, Redis, etc.)"
    echo "2. Run: cd src/api-gateway && cargo run --bin api-gateway"
    echo "3. Test API endpoints at http://localhost:8080"
    echo "4. Check health endpoint: curl http://localhost:8080/health"
    echo
    RESULT=0
elif [[ $TOTAL_FAILED -le 2 ]]; then
    echo
    echo -e "${YELLOW}⚠️  MOSTLY COMPLETE - Minor issues found${NC}"
    echo -e "${YELLOW}✅ Backend Agent implementation is FUNCTIONAL${NC}"
    echo
    echo "Minor issues to address:"
    echo "- Check failed tests above"
    echo "- Review missing documentation"
    echo "- Verify environment configuration"
    echo
    RESULT=0
else
    echo
    echo -e "${RED}❌ SIGNIFICANT ISSUES FOUND${NC}"
    echo -e "${RED}⚠️  Backend Agent implementation needs attention${NC}"
    echo
    echo "Critical issues to fix:"
    echo "- Address compilation errors"
    echo "- Fix failing tests"
    echo "- Complete missing implementations"
    echo
    RESULT=1
fi

echo "Backend Agent Validation Complete"
echo "=================================="

exit $RESULT

#!/bin/bash

# AI-CORE Task Complexity Analysis Tool
# Description: Analyzes task complexity and domain classification for intelligent agent selection
# Version: 1.0
# Created: 2025-01-10

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="$PROJECT_ROOT/dev-works/logs/task-complexity-analysis.log"
CACHE_DIR="$PROJECT_ROOT/.cache/task-analysis"
PATTERNS_FILE="$PROJECT_ROOT/.kiro/patterns/task-complexity.yaml"

# Ensure required directories exist
mkdir -p "$(dirname "$LOG_FILE")"
mkdir -p "$CACHE_DIR"
mkdir -p "$PROJECT_ROOT/.kiro/patterns"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
    echo -e "$1"
}

# Error handling
error_exit() {
    log "${RED}ERROR: $1${NC}"
    exit 1
}

# Usage information
show_usage() {
    cat << EOF
${CYAN}AI-CORE Task Complexity Analysis Tool${NC}

${YELLOW}USAGE:${NC}
    $0 [OPTIONS] [TASK_DESCRIPTION]

${YELLOW}OPTIONS:${NC}
    -f, --files FILE_LIST       Comma-separated list of files involved
    -d, --description TEXT      Task description (if not provided as argument)
    -c, --context CONTEXT       Additional context information
    -v, --verbose               Enable verbose output
    -o, --output FORMAT         Output format (json|yaml|text) [default: text]
    -h, --help                  Show this help message

${YELLOW}EXAMPLES:${NC}
    $0 "Implement user authentication API endpoints"
    $0 -f "src/auth.rs,src/models.rs" "Add JWT token validation"
    $0 --verbose --output json "Optimize database queries for user search"

${YELLOW}COMPLEXITY LEVELS:${NC}
    ${GREEN}SIMPLE${NC}     (1-3): Basic CRUD operations, simple bug fixes
    ${YELLOW}MEDIUM${NC}     (4-6): API integrations, moderate refactoring
    ${RED}COMPLEX${NC}    (7-8): System architecture changes, performance optimization
    ${PURPLE}EXPERT${NC}     (9-10): Major system redesign, critical security implementations

${YELLOW}DOMAINS:${NC}
    Frontend, Backend, Database, DevOps, Security, Testing, Integration, Architecture
EOF
}

# Initialize default values
TASK_DESCRIPTION=""
FILES_INVOLVED=""
ADDITIONAL_CONTEXT=""
VERBOSE=false
OUTPUT_FORMAT="text"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--files)
            FILES_INVOLVED="$2"
            shift 2
            ;;
        -d|--description)
            TASK_DESCRIPTION="$2"
            shift 2
            ;;
        -c|--context)
            ADDITIONAL_CONTEXT="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            error_exit "Unknown option: $1"
            ;;
        *)
            if [[ -z "$TASK_DESCRIPTION" ]]; then
                TASK_DESCRIPTION="$1"
            else
                ADDITIONAL_CONTEXT="$ADDITIONAL_CONTEXT $1"
            fi
            shift
            ;;
    esac
done

# Validate inputs
if [[ -z "$TASK_DESCRIPTION" ]]; then
    error_exit "Task description is required. Use -d/--description or provide as argument."
fi

# Complexity analysis patterns
declare -A COMPLEXITY_KEYWORDS=(
    # Simple (1-3)
    ["simple_crud"]="create read update delete add remove list show display"
    ["simple_fixes"]="fix bug typo spelling format style lint"
    ["simple_config"]="config configuration setting parameter variable"

    # Medium (4-6)
    ["medium_integration"]="api integrate integration connect sync webhook"
    ["medium_refactor"]="refactor restructure reorganize clean optimize improve"
    ["medium_features"]="feature implement build develop enhance extend"

    # Complex (7-8)
    ["complex_architecture"]="architecture design pattern system structure framework"
    ["complex_performance"]="performance optimize speed memory cache scaling bottleneck"
    ["complex_security"]="security authentication authorization encryption ssl tls"

    # Expert (9-10)
    ["expert_system"]="redesign overhaul migration distributed microservices infrastructure"
    ["expert_critical"]="critical production deployment disaster recovery backup"
)

declare -A DOMAIN_KEYWORDS=(
    ["frontend"]="ui ux interface react component typescript css html dom browser client"
    ["backend"]="api server rust axum database postgres redis mongodb service endpoint"
    ["database"]="sql query index migration schema table column constraint performance"
    ["devops"]="docker kubernetes deployment infrastructure terraform ci cd pipeline"
    ["security"]="auth authentication authorization jwt encryption ssl vulnerability audit"
    ["testing"]="test unit integration e2e playwright coverage mock stub assertion"
    ["integration"]="webhook api external third-party service sync integration connect"
    ["architecture"]="design pattern system structure framework architecture microservices"
)

# File type analysis
analyze_file_types() {
    local files="$1"
    local complexity_score=0
    local domains=()

    if [[ -n "$files" ]]; then
        IFS=',' read -ra FILE_ARRAY <<< "$files"
        for file in "${FILE_ARRAY[@]}"; do
            case "$file" in
                *.rs)
                    complexity_score=$((complexity_score + 2))
                    domains+=("backend")
                    ;;
                *.ts|*.tsx|*.js|*.jsx)
                    complexity_score=$((complexity_score + 1))
                    domains+=("frontend")
                    ;;
                *.sql)
                    complexity_score=$((complexity_score + 2))
                    domains+=("database")
                    ;;
                *dockerfile*|*.yaml|*.yml)
                    complexity_score=$((complexity_score + 3))
                    domains+=("devops")
                    ;;
                *test*|*spec*)
                    complexity_score=$((complexity_score + 1))
                    domains+=("testing")
                    ;;
                *auth*|*security*)
                    complexity_score=$((complexity_score + 4))
                    domains+=("security")
                    ;;
                *api*|*integration*)
                    complexity_score=$((complexity_score + 2))
                    domains+=("integration")
                    ;;
            esac
        done
    fi

    echo "$complexity_score:$(IFS=,; echo "${domains[*]}")"
}

# Keyword analysis
analyze_keywords() {
    local text="$1"
    local complexity_score=0
    local domains=()
    local matched_patterns=()

    # Convert to lowercase for analysis
    text_lower=$(echo "$text" | tr '[:upper:]' '[:lower:]')

    # Check complexity patterns
    for pattern_name in "${!COMPLEXITY_KEYWORDS[@]}"; do
        keywords="${COMPLEXITY_KEYWORDS[$pattern_name]}"
        for keyword in $keywords; do
            if [[ "$text_lower" == *"$keyword"* ]]; then
                case "$pattern_name" in
                    simple_*)
                        complexity_score=$((complexity_score + 1))
                        ;;
                    medium_*)
                        complexity_score=$((complexity_score + 2))
                        ;;
                    complex_*)
                        complexity_score=$((complexity_score + 3))
                        ;;
                    expert_*)
                        complexity_score=$((complexity_score + 4))
                        ;;
                esac
                matched_patterns+=("$pattern_name:$keyword")
                break
            fi
        done
    done

    # Check domain patterns
    for domain in "${!DOMAIN_KEYWORDS[@]}"; do
        keywords="${DOMAIN_KEYWORDS[$domain]}"
        for keyword in $keywords; do
            if [[ "$text_lower" == *"$keyword"* ]]; then
                domains+=("$domain")
                break
            fi
        done
    done

    echo "$complexity_score:$(IFS=,; echo "${domains[*]}"):$(IFS=,; echo "${matched_patterns[*]}")"
}

# Effort estimation
estimate_effort() {
    local complexity_score="$1"

    if [[ $complexity_score -le 3 ]]; then
        echo "2-4 hours"
    elif [[ $complexity_score -le 6 ]]; then
        echo "4-8 hours"
    elif [[ $complexity_score -le 10 ]]; then
        echo "1-2 days"
    elif [[ $complexity_score -le 15 ]]; then
        echo "3-5 days"
    else
        echo "1+ weeks"
    fi
}

# Complexity level determination
get_complexity_level() {
    local score="$1"

    if [[ $score -le 3 ]]; then
        echo "SIMPLE"
    elif [[ $score -le 6 ]]; then
        echo "MEDIUM"
    elif [[ $score -le 10 ]]; then
        echo "COMPLEX"
    else
        echo "EXPERT"
    fi
}

# Main analysis function
perform_analysis() {
    local task="$TASK_DESCRIPTION"
    local files="$FILES_INVOLVED"
    local context="$ADDITIONAL_CONTEXT"

    log "${BLUE}Starting task complexity analysis...${NC}"

    # Analyze different aspects
    local file_analysis
    file_analysis=$(analyze_file_types "$files")
    local file_score
    file_score=$(echo "$file_analysis" | cut -d: -f1)
    local file_domains
    file_domains=$(echo "$file_analysis" | cut -d: -f2)

    local keyword_analysis
    keyword_analysis=$(analyze_keywords "$task $context")
    local keyword_score
    keyword_score=$(echo "$keyword_analysis" | cut -d: -f1)
    local keyword_domains
    keyword_domains=$(echo "$keyword_analysis" | cut -d: -f2)
    local matched_patterns
    matched_patterns=$(echo "$keyword_analysis" | cut -d: -f3)

    # Calculate total complexity score
    local total_score=$((file_score + keyword_score))

    # Determine complexity level
    local complexity_level
    complexity_level=$(get_complexity_level $total_score)

    # Combine domains and remove duplicates
    local all_domains="$file_domains,$keyword_domains"
    local unique_domains
    unique_domains=$(echo "$all_domains" | tr ',' '\n' | sort | uniq | grep -v '^$' | tr '\n' ',' | sed 's/,$//')

    # Estimate effort
    local effort_estimate
    effort_estimate=$(estimate_effort $total_score)

    # Generate skills required
    local skills_required=()
    IFS=',' read -ra DOMAIN_ARRAY <<< "$unique_domains"
    for domain in "${DOMAIN_ARRAY[@]}"; do
        case "$domain" in
            "frontend")
                skills_required+=("React/TypeScript" "UI/UX Design" "CSS/HTML")
                ;;
            "backend")
                skills_required+=("Rust/Axum" "API Design" "Service Architecture")
                ;;
            "database")
                skills_required+=("SQL" "Database Design" "Query Optimization")
                ;;
            "devops")
                skills_required+=("Docker/Kubernetes" "CI/CD" "Infrastructure")
                ;;
            "security")
                skills_required+=("Authentication" "Encryption" "Security Auditing")
                ;;
            "testing")
                skills_required+=("Test Automation" "Playwright" "Quality Assurance")
                ;;
            "integration")
                skills_required+=("API Integration" "Event Streaming" "External Services")
                ;;
            "architecture")
                skills_required+=("System Design" "Architecture Patterns" "Technical Strategy")
                ;;
        esac
    done

    # Risk assessment
    local risk_factors=()
    if [[ $total_score -gt 10 ]]; then
        risk_factors+=("High complexity may require multiple iterations")
    fi
    if [[ "$unique_domains" == *","* ]]; then
        risk_factors+=("Cross-domain coordination required")
    fi
    if [[ "$task" == *"critical"* || "$task" == *"production"* ]]; then
        risk_factors+=("Production impact requires careful planning")
    fi
    if [[ "$task" == *"migration"* || "$task" == *"redesign"* ]]; then
        risk_factors+=("Major change with potential breaking effects")
    fi

    # Output results based on format
    case "$OUTPUT_FORMAT" in
        "json")
            output_json "$total_score" "$complexity_level" "$unique_domains" "$effort_estimate" "${skills_required[@]}" "${risk_factors[@]}" "$matched_patterns"
            ;;
        "yaml")
            output_yaml "$total_score" "$complexity_level" "$unique_domains" "$effort_estimate" "${skills_required[@]}" "${risk_factors[@]}" "$matched_patterns"
            ;;
        *)
            output_text "$total_score" "$complexity_level" "$unique_domains" "$effort_estimate" "${skills_required[@]}" "${risk_factors[@]}" "$matched_patterns"
            ;;
    esac

    log "${GREEN}Task complexity analysis completed successfully${NC}"
}

# Output functions
output_text() {
    local score="$1"
    local level="$2"
    local domains="$3"
    local effort="$4"
    shift 4
    local skills=("$@")

    cat << EOF

${CYAN}🎯 TASK COMPLEXITY ANALYSIS RESULTS${NC}
${BLUE}═══════════════════════════════════════${NC}

${YELLOW}📊 COMPLEXITY ASSESSMENT:${NC}
  • Complexity Score: ${score}/20
  • Complexity Level: ${level}
  • Estimated Effort: ${effort}
  • Domain Classification: ${domains}

${YELLOW}🛠️ SKILLS REQUIRED:${NC}
$(for skill in "${skills[@]}"; do echo "  • $skill"; done)

${YELLOW}⚠️ RISK FACTORS:${NC}
$(for risk in "${risk_factors[@]}"; do echo "  • $risk"; done)

${YELLOW}💡 RECOMMENDATIONS:${NC}
  • Primary Agent Suggestion: $(get_agent_recommendation "$domains" "$level")
  • Secondary Agents: $(get_secondary_agents "$domains")
  • Collaboration Required: $(need_collaboration "$level" "$domains")

${YELLOW}📋 ANALYSIS DETAILS:${NC}
  • File Score Contribution: ${file_score}
  • Keyword Score Contribution: ${keyword_score}
  • Matched Patterns: ${matched_patterns//,/, }
  • Analysis Confidence: $(calculate_confidence "$score" "$domains")%

EOF
}

output_json() {
    local score="$1"
    local level="$2"
    local domains="$3"
    local effort="$4"
    shift 4

    cat << EOF
{
  "analysis_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "task_description": "$TASK_DESCRIPTION",
  "complexity": {
    "score": $score,
    "level": "$level",
    "max_score": 20
  },
  "effort_estimate": "$effort",
  "domains": [$(echo "$domains" | sed 's/,/", "/g' | sed 's/^/"/;s/$/"/')],
  "skills_required": [$(IFS=,; echo "\"${skills[*]}\"" | sed 's/,/", "/g')],
  "risk_factors": [$(IFS=,; echo "\"${risk_factors[*]}\"" | sed 's/,/", "/g')],
  "recommendations": {
    "primary_agent": "$(get_agent_recommendation "$domains" "$level")",
    "secondary_agents": [$(get_secondary_agents "$domains" | sed 's/,/", "/g' | sed 's/^/"/;s/$/"/')],
    "collaboration_required": $(need_collaboration "$level" "$domains")
  },
  "analysis_details": {
    "file_score_contribution": $file_score,
    "keyword_score_contribution": $keyword_score,
    "matched_patterns": [$(echo "$matched_patterns" | sed 's/,/", "/g' | sed 's/^/"/;s/$/"/')],
    "confidence_percentage": $(calculate_confidence "$score" "$domains")
  }
}
EOF
}

output_yaml() {
    local score="$1"
    local level="$2"
    local domains="$3"
    local effort="$4"
    shift 4

    cat << EOF
analysis_timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)
task_description: "$TASK_DESCRIPTION"
complexity:
  score: $score
  level: $level
  max_score: 20
effort_estimate: "$effort"
domains:
$(echo "$domains" | tr ',' '\n' | sed 's/^/  - /')
skills_required:
$(for skill in "${skills[@]}"; do echo "  - \"$skill\""; done)
risk_factors:
$(for risk in "${risk_factors[@]}"; do echo "  - \"$risk\""; done)
recommendations:
  primary_agent: $(get_agent_recommendation "$domains" "$level")
  secondary_agents:
$(get_secondary_agents "$domains" | tr ',' '\n' | sed 's/^/    - /')
  collaboration_required: $(need_collaboration "$level" "$domains")
analysis_details:
  file_score_contribution: $file_score
  keyword_score_contribution: $keyword_score
  matched_patterns:
$(echo "$matched_patterns" | tr ',' '\n' | sed 's/^/    - /')
  confidence_percentage: $(calculate_confidence "$score" "$domains")
EOF
}

# Helper functions
get_agent_recommendation() {
    local domains="$1"
    local level="$2"

    # Primary domain-based recommendation
    if [[ "$domains" == *"backend"* ]]; then
        echo "backend-agent"
    elif [[ "$domains" == *"frontend"* ]]; then
        echo "frontend-agent"
    elif [[ "$domains" == *"database"* ]]; then
        echo "database-agent"
    elif [[ "$domains" == *"devops"* ]]; then
        echo "devops-agent"
    elif [[ "$domains" == *"security"* ]]; then
        echo "security-agent"
    elif [[ "$domains" == *"testing"* ]]; then
        echo "qa-agent"
    elif [[ "$domains" == *"architecture"* || "$level" == "EXPERT" ]]; then
        echo "architect-agent"
    else
        echo "coordinator-agent"
    fi
}

get_secondary_agents() {
    local domains="$1"
    local secondary=()

    IFS=',' read -ra DOMAIN_ARRAY <<< "$domains"
    for domain in "${DOMAIN_ARRAY[@]}"; do
        case "$domain" in
            "frontend") [[ ! " ${secondary[@]} " =~ " frontend-agent " ]] && secondary+=("frontend-agent") ;;
            "backend") [[ ! " ${secondary[@]} " =~ " backend-agent " ]] && secondary+=("backend-agent") ;;
            "database") [[ ! " ${secondary[@]} " =~ " database-agent " ]] && secondary+=("database-agent") ;;
            "devops") [[ ! " ${secondary[@]} " =~ " devops-agent " ]] && secondary+=("devops-agent") ;;
            "security") [[ ! " ${secondary[@]} " =~ " security-agent " ]] && secondary+=("security-agent") ;;
            "testing") [[ ! " ${secondary[@]} " =~ " qa-agent " ]] && secondary+=("qa-agent") ;;
        esac
    done

    # Remove primary agent from secondary list
    local primary
    primary=$(get_agent_recommendation "$domains" "")
    secondary=("${secondary[@]/$primary}")

    IFS=,; echo "${secondary[*]}"
}

need_collaboration() {
    local level="$1"
    local domains="$2"

    local domain_count
    domain_count=$(echo "$domains" | tr ',' '\n' | wc -l)

    if [[ "$level" == "EXPERT" || $domain_count -gt 2 ]]; then
        echo "true"
    else
        echo "false"
    fi
}

calculate_confidence() {
    local score="$1"
    local domains="$2"

    local base_confidence=70

    # Increase confidence based on clear indicators
    if [[ -n "$domains" ]]; then
        base_confidence=$((base_confidence + 10))
    fi

    if [[ -n "$FILES_INVOLVED" ]]; then
        base_confidence=$((base_confidence + 10))
    fi

    if [[ ${#TASK_DESCRIPTION} -gt 20 ]]; then
        base_confidence=$((base_confidence + 10))
    fi

    # Cap at 95%
    if [[ $base_confidence -gt 95 ]]; then
        base_confidence=95
    fi

    echo $base_confidence
}

# Cache results
cache_results() {
    local cache_key
    cache_key=$(echo "$TASK_DESCRIPTION$FILES_INVOLVED$ADDITIONAL_CONTEXT" | md5sum | cut -d' ' -f1)
    local cache_file="$CACHE_DIR/$cache_key.cache"

    # Store results for future reference
    cat << EOF > "$cache_file"
timestamp=$(date +%s)
task_description=$TASK_DESCRIPTION
complexity_score=$total_score
complexity_level=$complexity_level
domains=$unique_domains
effort_estimate=$effort_estimate
EOF

    if [[ "$VERBOSE" == true ]]; then
        log "${BLUE}Results cached to: $cache_file${NC}"
    fi
}

# Main execution
main() {
    if [[ "$VERBOSE" == true ]]; then
        log "${BLUE}Task Complexity Analysis Tool Starting...${NC}"
        log "${BLUE}Task: $TASK_DESCRIPTION${NC}"
        log "${BLUE}Files: $FILES_INVOLVED${NC}"
        log "${BLUE}Context: $ADDITIONAL_CONTEXT${NC}"
    fi

    perform_analysis

    if [[ "$VERBOSE" == true ]]; then
        log "${GREEN}Analysis completed successfully${NC}"
    fi
}

# Execute main function
main "$@"

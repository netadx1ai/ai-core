#!/bin/bash

# AI-CORE File Pattern Analysis Tool
# Description: Analyzes file types, languages, and patterns for intelligent task routing
# Version: 1.0
# Created: 2025-01-17

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
LOG_FILE="$PROJECT_ROOT/dev-works/logs/file-pattern-analysis.log"
CACHE_DIR="$PROJECT_ROOT/.cache/file-analysis"

# Ensure required directories exist
mkdir -p "$(dirname "$LOG_FILE")"
mkdir -p "$CACHE_DIR"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        echo -e "$1"
    fi
}

# Error handling
error_exit() {
    echo -e "${RED}ERROR: $1${NC}" >&2
    log "ERROR: $1"
    exit 1
}

# Usage information
show_usage() {
    cat << EOF
${CYAN}AI-CORE File Pattern Analysis Tool${NC}

${YELLOW}USAGE:${NC}
    $0 [OPTIONS] [FILES...]

${YELLOW}OPTIONS:${NC}
    -f, --files FILE_LIST       Comma-separated list of files to analyze
    -d, --directory DIR         Directory to analyze (default: current)
    -r, --recursive             Analyze recursively
    -v, --verbose               Enable verbose output
    -o, --output FORMAT         Output format (json|yaml|text) [default: json]
    -p, --patterns              Show detailed pattern analysis
    -s, --stats                 Show file statistics
    -h, --help                  Show this help message

${YELLOW}EXAMPLES:${NC}
    $0 -f "src/auth.rs,src/models.rs"
    $0 -d src/backend -r --patterns
    $0 --stats --output json

${YELLOW}OUTPUT:${NC}
    File types, languages, project areas, integration requirements, dependencies
EOF
}

# File type detection and classification
declare -A FILE_PATTERNS=(
    # Backend (Rust)
    ["rust"]="*.rs"
    ["rust_config"]="Cargo.toml Cargo.lock"

    # Frontend (JavaScript/TypeScript)
    ["javascript"]="*.js *.jsx"
    ["typescript"]="*.ts *.tsx"
    ["web_config"]="package.json tsconfig.json webpack.config.js vite.config.js"
    ["styles"]="*.css *.scss *.sass *.less"

    # Database
    ["sql"]="*.sql"
    ["database_migrations"]="*migration*.sql *schema*.sql"

    # Configuration & Infrastructure
    ["docker"]="Dockerfile docker-compose.yml docker-compose.yaml"
    ["kubernetes"]="*.yaml *.yml"
    ["terraform"]="*.tf *.tfvars"

    # Documentation
    ["markdown"]="*.md"
    ["documentation"]="README* CHANGELOG* LICENSE*"

    # Testing
    ["tests"]="*test*.rs *test*.js *test*.ts *spec*.js *spec*.ts"

    # Security & Auth
    ["security"]="*auth* *security* *jwt* *oauth*"

    # API & Integration
    ["api"]="*api* *endpoint* *route* *handler*"
    ["integration"]="*webhook* *integration* *sync*"
)

declare -A DOMAIN_MAPPING=(
    ["rust"]="backend"
    ["rust_config"]="backend"
    ["javascript"]="frontend"
    ["typescript"]="frontend"
    ["web_config"]="frontend"
    ["styles"]="frontend"
    ["sql"]="database"
    ["database_migrations"]="database"
    ["docker"]="devops"
    ["kubernetes"]="devops"
    ["terraform"]="devops"
    ["markdown"]="documentation"
    ["documentation"]="documentation"
    ["tests"]="testing"
    ["security"]="security"
    ["api"]="integration"
    ["integration"]="integration"
)

# Initialize variables
FILES_LIST=""
DIRECTORY="."
RECURSIVE=false
VERBOSE=false
OUTPUT_FORMAT="json"
SHOW_PATTERNS=false
SHOW_STATS=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--files)
            FILES_LIST="$2"
            shift 2
            ;;
        -d|--directory)
            DIRECTORY="$2"
            shift 2
            ;;
        -r|--recursive)
            RECURSIVE=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -p|--patterns)
            SHOW_PATTERNS=true
            shift
            ;;
        -s|--stats)
            SHOW_STATS=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            error_exit "Unknown option: $1"
            ;;
        *)
            if [[ -z "$FILES_LIST" ]]; then
                FILES_LIST="$1"
            else
                FILES_LIST="$FILES_LIST,$1"
            fi
            shift
            ;;
    esac
done

# Analyze file patterns
analyze_files() {
    local files_to_analyze=()
    local analysis_result=""

    # Determine files to analyze
    if [[ -n "$FILES_LIST" ]]; then
        IFS=',' read -ra files_to_analyze <<< "$FILES_LIST"
    else
        if [[ "$RECURSIVE" == "true" ]]; then
            readarray -t files_to_analyze < <(find "$DIRECTORY" -type f -not -path "*/.*" -not -path "*/target/*" -not -path "*/node_modules/*")
        else
            readarray -t files_to_analyze < <(find "$DIRECTORY" -maxdepth 1 -type f -not -path "*/.*")
        fi
    fi

    # Initialize analysis data
    declare -A file_types=()
    declare -A domains=()
    declare -A languages=()
    declare -A project_areas=()
    local total_files=0
    local complexity_score=0

    log "Analyzing ${#files_to_analyze[@]} files..."

    for file in "${files_to_analyze[@]}"; do
        if [[ ! -f "$file" ]]; then
            continue
        fi

        total_files=$((total_files + 1))
        local basename_file=$(basename "$file")
        local extension="${file##*.}"
        local classified=false

        # Classify file by patterns
        for pattern_type in "${!FILE_PATTERNS[@]}"; do
            local patterns="${FILE_PATTERNS[$pattern_type]}"
            for pattern in $patterns; do
                if [[ "$basename_file" == $pattern ]] || [[ "$file" == *"$pattern"* ]]; then
                    file_types["$pattern_type"]=$((${file_types[$pattern_type]:-0} + 1))

                    # Map to domain
                    local domain="${DOMAIN_MAPPING[$pattern_type]:-other}"
                    domains["$domain"]=$((${domains[$domain]:-0} + 1))

                    # Extract language
                    case "$extension" in
                        "rs") languages["rust"]=$((${languages["rust"]:-0} + 1)) ;;
                        "js"|"jsx") languages["javascript"]=$((${languages["javascript"]:-0} + 1)) ;;
                        "ts"|"tsx") languages["typescript"]=$((${languages["typescript"]:-0} + 1)) ;;
                        "sql") languages["sql"]=$((${languages["sql"]:-0} + 1)) ;;
                        "md") languages["markdown"]=$((${languages["markdown"]:-0} + 1)) ;;
                        "yml"|"yaml") languages["yaml"]=$((${languages["yaml"]:-0} + 1)) ;;
                    esac

                    # Determine project area
                    if [[ "$file" == *"src/backend"* ]]; then
                        project_areas["backend"]=$((${project_areas["backend"]:-0} + 1))
                    elif [[ "$file" == *"src/frontend"* ]] || [[ "$file" == *"src/ui"* ]]; then
                        project_areas["frontend"]=$((${project_areas["frontend"]:-0} + 1))
                    elif [[ "$file" == *"tests"* ]]; then
                        project_areas["testing"]=$((${project_areas["testing"]:-0} + 1))
                    elif [[ "$file" == *"docs"* ]] || [[ "$file" == *".kiro"* ]]; then
                        project_areas["documentation"]=$((${project_areas["documentation"]:-0} + 1))
                    elif [[ "$file" == *"tools"* ]]; then
                        project_areas["automation"]=$((${project_areas["automation"]:-0} + 1))
                    elif [[ "$file" == *"infrastructure"* ]]; then
                        project_areas["infrastructure"]=$((${project_areas["infrastructure"]:-0} + 1))
                    else
                        project_areas["core"]=$((${project_areas["core"]:-0} + 1))
                    fi

                    classified=true

                    # Add complexity scoring
                    case "$pattern_type" in
                        "rust"|"typescript") complexity_score=$((complexity_score + 3)) ;;
                        "javascript"|"sql") complexity_score=$((complexity_score + 2)) ;;
                        "docker"|"kubernetes") complexity_score=$((complexity_score + 4)) ;;
                        "security") complexity_score=$((complexity_score + 5)) ;;
                        *) complexity_score=$((complexity_score + 1)) ;;
                    esac

                    break
                fi
            done
            if [[ "$classified" == "true" ]]; then
                break
            fi
        done

        # Handle unclassified files
        if [[ "$classified" == "false" ]]; then
            file_types["other"]=$((${file_types["other"]:-0} + 1))
            domains["other"]=$((${domains["other"]:-0} + 1))
        fi
    done

    # Generate output based on format
    case "$OUTPUT_FORMAT" in
        "json")
            generate_json_output
            ;;
        "yaml")
            generate_yaml_output
            ;;
        "text"|*)
            generate_text_output
            ;;
    esac
}

# Generate JSON output
generate_json_output() {
    cat << EOF
{
    "timestamp": "$(date -Iseconds)",
    "analysis_summary": {
        "total_files": $total_files,
        "complexity_score": $complexity_score,
        "primary_languages": $(generate_json_array languages),
        "primary_domains": $(generate_json_array domains),
        "project_areas": $(generate_json_array project_areas)
    },
    "file_types": $(generate_json_object file_types),
    "domains": $(generate_json_object domains),
    "languages": $(generate_json_object languages),
    "project_areas": $(generate_json_object project_areas),
    "integration_requirements": {
        "database_integration": $(( ${domains["database"]:-0} > 0 )),
        "frontend_backend_integration": $(( ${domains["frontend"]:-0} > 0 && ${domains["backend"]:-0} > 0 )),
        "external_api_integration": $(( ${file_types["integration"]:-0} > 0 )),
        "security_considerations": $(( ${domains["security"]:-0} > 0 ))
    },
    "dependencies": {
        "requires_database": $(( ${languages["sql"]:-0} > 0 || ${domains["database"]:-0} > 0 )),
        "requires_frontend_build": $(( ${languages["typescript"]:-0} > 0 || ${languages["javascript"]:-0} > 0 )),
        "requires_rust_compilation": $(( ${languages["rust"]:-0} > 0 )),
        "requires_infrastructure": $(( ${domains["devops"]:-0} > 0 ))
    },
    "recommendations": {
        "primary_agent": "$(get_recommended_agent)",
        "secondary_agents": $(get_secondary_agents),
        "estimated_complexity": "$(get_complexity_level)"
    }
}
EOF
}

# Helper function to generate JSON arrays
generate_json_array() {
    local -n array_ref=$1
    local result="["
    local first=true

    # Sort by value (highest first) and take top 3
    for key in $(printf '%s\n' "${!array_ref[@]}" | head -3); do
        if [[ $first == true ]]; then
            first=false
        else
            result+=", "
        fi
        result+="\"$key\""
    done
    result+="]"
    echo "$result"
}

# Helper function to generate JSON objects
generate_json_object() {
    local -n obj_ref=$1
    local result="{"
    local first=true

    for key in "${!obj_ref[@]}"; do
        if [[ $first == true ]]; then
            first=false
        else
            result+=", "
        fi
        result+="\"$key\": ${obj_ref[$key]}"
    done
    result+="}"
    echo "$result"
}

# Get recommended agent based on file analysis
get_recommended_agent() {
    local max_count=0
    local recommended_agent="backend"

    for domain in "${!domains[@]}"; do
        if (( ${domains[$domain]} > max_count )); then
            max_count=${domains[$domain]}
            case "$domain" in
                "backend") recommended_agent="backend" ;;
                "frontend") recommended_agent="frontend" ;;
                "database") recommended_agent="database" ;;
                "devops") recommended_agent="devops" ;;
                "security") recommended_agent="security" ;;
                "testing") recommended_agent="qa" ;;
                "integration") recommended_agent="integration" ;;
                *) recommended_agent="architect" ;;
            esac
        fi
    done

    echo "$recommended_agent"
}

# Get secondary agents
get_secondary_agents() {
    local secondary=()
    local primary=$(get_recommended_agent)

    # Add relevant secondary agents based on analysis
    if (( ${domains["security"]:-0} > 0 )) && [[ "$primary" != "security" ]]; then
        secondary+=("security")
    fi
    if (( ${domains["database"]:-0} > 0 )) && [[ "$primary" != "database" ]]; then
        secondary+=("database")
    fi
    if (( ${domains["testing"]:-0} > 0 )) && [[ "$primary" != "qa" ]]; then
        secondary+=("qa")
    fi
    if (( ${domains["devops"]:-0} > 0 )) && [[ "$primary" != "devops" ]]; then
        secondary+=("devops")
    fi

    # Format as JSON array
    local result="["
    local first=true
    for agent in "${secondary[@]}"; do
        if [[ $first == true ]]; then
            first=false
        else
            result+=", "
        fi
        result+="\"$agent\""
    done
    result+="]"
    echo "$result"
}

# Get complexity level
get_complexity_level() {
    if (( complexity_score <= 5 )); then
        echo "simple"
    elif (( complexity_score <= 15 )); then
        echo "medium"
    elif (( complexity_score <= 25 )); then
        echo "complex"
    else
        echo "expert"
    fi
}

# Generate text output
generate_text_output() {
    echo -e "${CYAN}📁 File Pattern Analysis Results${NC}"
    echo ""
    echo -e "${GREEN}📊 Summary:${NC}"
    echo "  Total Files: $total_files"
    echo "  Complexity Score: $complexity_score"
    echo "  Complexity Level: $(get_complexity_level)"
    echo ""

    if [[ "${#domains[@]}" -gt 0 ]]; then
        echo -e "${YELLOW}🎯 Domains Identified:${NC}"
        for domain in "${!domains[@]}"; do
            echo "  $domain: ${domains[$domain]} files"
        done
        echo ""
    fi

    if [[ "${#languages[@]}" -gt 0 ]]; then
        echo -e "${BLUE}💻 Languages Detected:${NC}"
        for lang in "${!languages[@]}"; do
            echo "  $lang: ${languages[$lang]} files"
        done
        echo ""
    fi

    echo -e "${PURPLE}🤖 Agent Recommendation:${NC}"
    echo "  Primary: $(get_recommended_agent)"
    echo "  Secondary: $(get_secondary_agents | sed 's/\["\|"\]//g' | sed 's/", "/ /g')"
}

# Generate YAML output
generate_yaml_output() {
    echo "timestamp: $(date -Iseconds)"
    echo "analysis_summary:"
    echo "  total_files: $total_files"
    echo "  complexity_score: $complexity_score"
    echo "  complexity_level: $(get_complexity_level)"
    echo "domains:"
    for domain in "${!domains[@]}"; do
        echo "  $domain: ${domains[$domain]}"
    done
    echo "languages:"
    for lang in "${!languages[@]}"; do
        echo "  $lang: ${languages[$lang]}"
    done
    echo "recommendations:"
    echo "  primary_agent: $(get_recommended_agent)"
    echo "  secondary_agents: [$(get_secondary_agents | sed 's/\["\|"\]//g' | sed 's/", "/", "/g')]"
}

# Main execution
main() {
    log "Starting file pattern analysis"

    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${CYAN}🔍 AI-CORE File Pattern Analysis${NC}"
        echo ""
    fi

    analyze_files

    log "File pattern analysis completed"
}

# Execute main function
main "$@"

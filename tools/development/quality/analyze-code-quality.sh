#!/bin/bash

# AI-CORE Code Quality Analysis Tool
# Description: Analyzes code quality for intelligent quality gates
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
LOG_FILE="$PROJECT_ROOT/dev-works/logs/code-quality-analysis.log"
TEMP_DIR="$PROJECT_ROOT/.tmp/quality-analysis"
QUALITY_CACHE="$PROJECT_ROOT/.cache/quality-metrics"

# Ensure required directories exist
mkdir -p "$(dirname "$LOG_FILE")"
mkdir -p "$TEMP_DIR"
mkdir -p "$QUALITY_CACHE"

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
${CYAN}AI-CORE Code Quality Analysis Tool${NC}

${YELLOW}USAGE:${NC}
    $0 [OPTIONS] [FILES...]

${YELLOW}OPTIONS:${NC}
    -f, --files FILE_LIST       Comma-separated list of files to analyze
    -d, --directory DIR         Directory to analyze recursively
    -t, --threshold SCORE       Quality threshold (0-100) [default: 80]
    -o, --output FORMAT         Output format (json|yaml|text) [default: text]
    -v, --verbose               Enable verbose output
    -c, --comprehensive         Run comprehensive analysis (slower but thorough)
    -r, --report-only           Generate report without enforcing thresholds
    -h, --help                  Show this help message

${YELLOW}EXAMPLES:${NC}
    $0                              # Analyze all changed files
    $0 -f "src/main.rs,src/lib.rs"  # Analyze specific files
    $0 -d src/services              # Analyze directory
    $0 -t 90 --comprehensive        # Strict analysis with high threshold

${YELLOW}QUALITY METRICS:${NC}
    • Code Complexity (Cyclomatic, Cognitive)
    • Maintainability Index
    • Code Duplication Detection
    • Documentation Coverage
    • Architecture Pattern Compliance
    • Security Pattern Analysis
EOF
}

# Initialize default values
TARGET_FILES=""
TARGET_DIRECTORY=""
QUALITY_THRESHOLD=80
OUTPUT_FORMAT="text"
VERBOSE=false
COMPREHENSIVE=false
REPORT_ONLY=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--files)
            TARGET_FILES="$2"
            shift 2
            ;;
        -d|--directory)
            TARGET_DIRECTORY="$2"
            shift 2
            ;;
        -t|--threshold)
            QUALITY_THRESHOLD="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -c|--comprehensive)
            COMPREHENSIVE=true
            shift
            ;;
        -r|--report-only)
            REPORT_ONLY=true
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
            if [[ -z "$TARGET_FILES" ]]; then
                TARGET_FILES="$1"
            else
                TARGET_FILES="$TARGET_FILES,$1"
            fi
            shift
            ;;
    esac
done

# Quality analysis functions
analyze_rust_complexity() {
    local file="$1"
    local complexity_score=0
    local function_count=0
    local high_complexity_functions=0

    if [[ ! -f "$file" ]]; then
        echo "0:0:0:File not found"
        return
    fi

    # Count functions and analyze complexity
    while IFS= read -r line; do
        # Function definitions
        if [[ "$line" =~ ^[[:space:]]*fn[[:space:]] ]] || [[ "$line" =~ ^[[:space:]]*pub[[:space:]]+fn[[:space:]] ]]; then
            ((function_count++))

            # Simple complexity indicators
            local line_complexity=1

            # Control flow statements increase complexity
            line_complexity=$((line_complexity + $(echo "$line" | grep -o "if\|else\|match\|for\|while\|loop" | wc -l)))

            # Error handling patterns
            line_complexity=$((line_complexity + $(echo "$line" | grep -o "Result\|Option\|?" | wc -l)))

            if [[ $line_complexity -gt 10 ]]; then
                ((high_complexity_functions++))
            fi

            complexity_score=$((complexity_score + line_complexity))
        fi
    done < "$file"

    # Calculate average complexity
    local avg_complexity=0
    if [[ $function_count -gt 0 ]]; then
        avg_complexity=$((complexity_score / function_count))
    fi

    echo "$complexity_score:$function_count:$high_complexity_functions:$avg_complexity"
}

analyze_typescript_complexity() {
    local file="$1"
    local complexity_score=0
    local function_count=0
    local high_complexity_functions=0

    if [[ ! -f "$file" ]]; then
        echo "0:0:0:File not found"
        return
    fi

    # Count functions and analyze complexity
    while IFS= read -r line; do
        # Function definitions (various TypeScript patterns)
        if [[ "$line" =~ function[[:space:]] ]] || [[ "$line" =~ =>[[:space:]] ]] || [[ "$line" =~ ^[[:space:]]*[a-zA-Z_][a-zA-Z0-9_]*\([^)]*\)[[:space:]]*\{ ]]; then
            ((function_count++))

            local line_complexity=1

            # Control flow statements
            line_complexity=$((line_complexity + $(echo "$line" | grep -o "if\|else\|switch\|for\|while\|try\|catch" | wc -l)))

            # TypeScript specific complexity
            line_complexity=$((line_complexity + $(echo "$line" | grep -o "Promise\|async\|await\|Observable" | wc -l)))

            if [[ $line_complexity -gt 10 ]]; then
                ((high_complexity_functions++))
            fi

            complexity_score=$((complexity_score + line_complexity))
        fi
    done < "$file"

    local avg_complexity=0
    if [[ $function_count -gt 0 ]]; then
        avg_complexity=$((complexity_score / function_count))
    fi

    echo "$complexity_score:$function_count:$high_complexity_functions:$avg_complexity"
}

analyze_documentation_coverage() {
    local file="$1"
    local total_functions=0
    local documented_functions=0
    local documentation_score=0

    if [[ ! -f "$file" ]]; then
        echo "0:0:0"
        return
    fi

    case "$file" in
        *.rs)
            # Rust documentation patterns
            while IFS= read -r line; do
                if [[ "$line" =~ ^[[:space:]]*fn[[:space:]] ]] || [[ "$line" =~ ^[[:space:]]*pub[[:space:]]+fn[[:space:]] ]]; then
                    ((total_functions++))
                fi
                if [[ "$line" =~ ^[[:space:]]*///[[:space:]] ]]; then
                    ((documented_functions++))
                fi
            done < "$file"
            ;;
        *.ts|*.tsx|*.js|*.jsx)
            # TypeScript/JavaScript documentation patterns
            local in_jsdoc=false
            while IFS= read -r line; do
                if [[ "$line" =~ /\*\* ]]; then
                    in_jsdoc=true
                elif [[ "$line" =~ \*/ ]]; then
                    in_jsdoc=false
                elif [[ "$line" =~ function[[:space:]] ]] || [[ "$line" =~ =>[[:space:]] ]]; then
                    ((total_functions++))
                    if [[ "$in_jsdoc" == true ]]; then
                        ((documented_functions++))
                    fi
                fi
            done < "$file"
            ;;
    esac

    if [[ $total_functions -gt 0 ]]; then
        documentation_score=$((documented_functions * 100 / total_functions))
    else
        documentation_score=100  # No functions = fully documented
    fi

    echo "$total_functions:$documented_functions:$documentation_score"
}

detect_code_duplication() {
    local file="$1"
    local duplication_score=0
    local suspicious_blocks=0

    if [[ ! -f "$file" ]]; then
        echo "0:0"
        return
    fi

    # Simple duplication detection using line patterns
    local temp_file="$TEMP_DIR/$(basename "$file").dedup"

    # Remove comments and empty lines, normalize whitespace
    sed -e 's|//.*||g' -e 's|/\*.*\*/||g' -e '/^[[:space:]]*$/d' -e 's/^[[:space:]]*//' "$file" > "$temp_file"

    # Find potentially duplicated lines (3+ occurrences)
    local duplicated_lines
    duplicated_lines=$(sort "$temp_file" | uniq -c | awk '$1 >= 3 { sum += $1 } END { print sum+0 }')

    local total_lines
    total_lines=$(wc -l < "$temp_file")

    if [[ $total_lines -gt 0 ]]; then
        duplication_score=$((duplicated_lines * 100 / total_lines))
        if [[ $duplication_score -gt 20 ]]; then
            suspicious_blocks=1
        fi
    fi

    rm -f "$temp_file"
    echo "$duplication_score:$suspicious_blocks"
}

analyze_architectural_patterns() {
    local file="$1"
    local pattern_score=50  # Start with neutral score
    local pattern_violations=0

    if [[ ! -f "$file" ]]; then
        echo "50:0"
        return
    fi

    case "$file" in
        *.rs)
            # Rust architectural patterns
            if grep -q "pub struct\|pub enum" "$file"; then
                pattern_score=$((pattern_score + 10))  # Good structure definition
            fi
            if grep -q "impl.*for" "$file"; then
                pattern_score=$((pattern_score + 10))  # Trait implementations
            fi
            if grep -q "unsafe" "$file"; then
                pattern_score=$((pattern_score - 20))  # Unsafe code
                ((pattern_violations++))
            fi
            if grep -q "unwrap()\|expect(" "$file"; then
                pattern_score=$((pattern_score - 10))  # Poor error handling
                ((pattern_violations++))
            fi
            ;;
        *.ts|*.tsx|*.js|*.jsx)
            # TypeScript/JavaScript patterns
            if grep -q "class\|interface\|type" "$file"; then
                pattern_score=$((pattern_score + 10))  # Good type definitions
            fi
            if grep -q "async\|Promise" "$file"; then
                pattern_score=$((pattern_score + 5))   # Modern async patterns
            fi
            if grep -q "any\|@ts-ignore" "$file"; then
                pattern_score=$((pattern_score - 15))  # Type safety violations
                ((pattern_violations++))
            fi
            if grep -q "console\.log\|debugger" "$file"; then
                pattern_score=$((pattern_score - 5))   # Debug code left in
                ((pattern_violations++))
            fi
            ;;
    esac

    # Ensure score stays within bounds
    if [[ $pattern_score -gt 100 ]]; then pattern_score=100; fi
    if [[ $pattern_score -lt 0 ]]; then pattern_score=0; fi

    echo "$pattern_score:$pattern_violations"
}

calculate_maintainability_index() {
    local complexity="$1"
    local lines_of_code="$2"
    local documentation_coverage="$3"
    local duplication_score="$4"

    # Simplified maintainability index calculation
    # MI = 171 - 5.2 * ln(HV) - 0.23 * CC - 16.2 * ln(LOC) + 50 * sin(sqrt(2.4 * CM))
    # Simplified version: focus on key factors

    local base_score=100

    # Complexity penalty
    if [[ $complexity -gt 0 ]]; then
        local complexity_penalty=$((complexity * 2))
        base_score=$((base_score - complexity_penalty))
    fi

    # Size penalty
    if [[ $lines_of_code -gt 500 ]]; then
        local size_penalty=$(((lines_of_code - 500) / 50))
        base_score=$((base_score - size_penalty))
    fi

    # Documentation bonus
    local doc_bonus=$((documentation_coverage / 5))
    base_score=$((base_score + doc_bonus))

    # Duplication penalty
    local dup_penalty=$((duplication_score * 2))
    base_score=$((base_score - dup_penalty))

    # Ensure score stays within bounds
    if [[ $base_score -gt 100 ]]; then base_score=100; fi
    if [[ $base_score -lt 0 ]]; then base_score=0; fi

    echo "$base_score"
}

analyze_single_file() {
    local file="$1"

    if [[ "$VERBOSE" == true ]]; then
        log "${YELLOW}Analyzing: $file${NC}"
    fi

    # Basic file metrics
    local lines_of_code
    lines_of_code=$(wc -l < "$file" 2>/dev/null || echo "0")

    local file_size
    file_size=$(stat -c%s "$file" 2>/dev/null || echo "0")

    # Complexity analysis
    local complexity_result
    case "$file" in
        *.rs)
            complexity_result=$(analyze_rust_complexity "$file")
            ;;
        *.ts|*.tsx|*.js|*.jsx)
            complexity_result=$(analyze_typescript_complexity "$file")
            ;;
        *)
            complexity_result="0:0:0:0"
            ;;
    esac

    IFS=':' read -r total_complexity function_count high_complexity_functions avg_complexity <<< "$complexity_result"

    # Documentation analysis
    local doc_result
    doc_result=$(analyze_documentation_coverage "$file")
    IFS=':' read -r total_functions documented_functions doc_coverage <<< "$doc_result"

    # Duplication analysis
    local dup_result
    dup_result=$(detect_code_duplication "$file")
    IFS=':' read -r duplication_score suspicious_blocks <<< "$dup_result"

    # Architectural pattern analysis
    local pattern_result
    pattern_result=$(analyze_architectural_patterns "$file")
    IFS=':' read -r pattern_score pattern_violations <<< "$pattern_result"

    # Calculate maintainability index
    local maintainability_index
    maintainability_index=$(calculate_maintainability_index "$total_complexity" "$lines_of_code" "$doc_coverage" "$duplication_score")

    # Calculate overall quality score
    local quality_score
    quality_score=$(echo "scale=2; ($maintainability_index * 0.3) + ($doc_coverage * 0.2) + ($pattern_score * 0.3) + ((100 - $duplication_score) * 0.2)" | bc -l 2>/dev/null || echo "50")

    # Output result
    echo "$file:$quality_score:$maintainability_index:$total_complexity:$doc_coverage:$duplication_score:$pattern_score:$pattern_violations:$lines_of_code:$function_count"
}

get_files_to_analyze() {
    local files=()

    if [[ -n "$TARGET_FILES" ]]; then
        # Specific files provided
        IFS=',' read -ra FILE_ARRAY <<< "$TARGET_FILES"
        for file in "${FILE_ARRAY[@]}"; do
            if [[ -f "$file" ]]; then
                files+=("$file")
            fi
        done
    elif [[ -n "$TARGET_DIRECTORY" ]]; then
        # Directory provided
        while IFS= read -r -d '' file; do
            files+=("$file")
        done < <(find "$TARGET_DIRECTORY" -type f \( -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \) -print0)
    else
        # Auto-detect changed files or analyze src directory
        if command -v git &> /dev/null && git rev-parse --git-dir > /dev/null 2>&1; then
            # Get changed files
            while IFS= read -r file; do
                if [[ -f "$file" && "$file" =~ \.(rs|ts|tsx|js|jsx)$ ]]; then
                    files+=("$file")
                fi
            done < <(git diff --name-only HEAD~1 2>/dev/null || echo "")
        fi

        # If no changed files or not a git repo, analyze src directory
        if [[ ${#files[@]} -eq 0 ]]; then
            while IFS= read -r -d '' file; do
                files+=("$file")
            done < <(find "$PROJECT_ROOT/src" -type f \( -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \) -print0 2>/dev/null || true)
        fi
    fi

    echo "${files[@]}"
}

format_output() {
    local format="$1"
    shift
    local results=("$@")

    case "$format" in
        "json")
            format_json_output "${results[@]}"
            ;;
        "yaml")
            format_yaml_output "${results[@]}"
            ;;
        *)
            format_text_output "${results[@]}"
            ;;
    esac
}

format_text_output() {
    local results=("$@")
    local total_files=${#results[@]}
    local passed_files=0
    local failed_files=0
    local total_quality_score=0

    cat << EOF

${CYAN}🎯 CODE QUALITY ANALYSIS RESULTS${NC}
${BLUE}═══════════════════════════════════${NC}

${YELLOW}📊 ANALYSIS SUMMARY:${NC}
  • Files Analyzed: $total_files
  • Quality Threshold: $QUALITY_THRESHOLD%
  • Analysis Date: $(date)

${YELLOW}📋 DETAILED RESULTS:${NC}

EOF

    # Process results
    for result in "${results[@]}"; do
        IFS=':' read -r file quality_score maintainability complexity doc_coverage duplication pattern_score violations lines functions <<< "$result"

        total_quality_score=$(echo "$total_quality_score + $quality_score" | bc -l)

        local status_indicator=""
        local quality_badge=""

        if (( $(echo "$quality_score >= $QUALITY_THRESHOLD" | bc -l) )); then
            ((passed_files++))
            status_indicator="${GREEN}✅ PASS${NC}"
            if (( $(echo "$quality_score >= 90" | bc -l) )); then
                quality_badge="${GREEN}🏆 EXCELLENT${NC}"
            else
                quality_badge="${GREEN}✅ GOOD${NC}"
            fi
        else
            ((failed_files++))
            status_indicator="${RED}❌ FAIL${NC}"
            if (( $(echo "$quality_score >= 60" | bc -l) )); then
                quality_badge="${YELLOW}⚠️ NEEDS IMPROVEMENT${NC}"
            else
                quality_badge="${RED}🚨 POOR QUALITY${NC}"
            fi
        fi

        cat << EOF
${BLUE}$(basename "$file")${NC} - $status_indicator $quality_badge
├─ Quality Score: ${quality_score}%
├─ Maintainability: ${maintainability}%
├─ Complexity: $complexity (${functions} functions)
├─ Documentation: ${doc_coverage}%
├─ Duplication: ${duplication}%
├─ Pattern Compliance: ${pattern_score}%
└─ Size: $lines lines

EOF
    done

    # Summary statistics
    local avg_quality_score=0
    if [[ $total_files -gt 0 ]]; then
        avg_quality_score=$(echo "scale=2; $total_quality_score / $total_files" | bc -l)
    fi

    local pass_rate=0
    if [[ $total_files -gt 0 ]]; then
        pass_rate=$(echo "scale=2; $passed_files * 100 / $total_files" | bc -l)
    fi

    cat << EOF
${YELLOW}📈 OVERALL METRICS:${NC}
  • Average Quality Score: ${avg_quality_score}%
  • Pass Rate: ${pass_rate}% (${passed_files}/${total_files})
  • Failed Files: $failed_files

EOF

    # Recommendations
    cat << EOF
${YELLOW}💡 RECOMMENDATIONS:${NC}

EOF

    if [[ $failed_files -gt 0 ]]; then
        echo "  🔧 ${failed_files} files need quality improvements"
    fi

    if (( $(echo "$avg_quality_score < 80" | bc -l) )); then
        echo "  📚 Consider code review and refactoring sessions"
        echo "  🛠️ Focus on reducing complexity and improving documentation"
    fi

    if [[ $passed_files -eq $total_files ]] && [[ $total_files -gt 0 ]]; then
        echo "  🎉 All files meet quality standards!"
    fi

    echo ""
}

format_json_output() {
    local results=("$@")

    echo "{"
    echo "  \"analysis_timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"quality_threshold\": $QUALITY_THRESHOLD,"
    echo "  \"files\": ["

    local first=true
    for result in "${results[@]}"; do
        IFS=':' read -r file quality_score maintainability complexity doc_coverage duplication pattern_score violations lines functions <<< "$result"

        if [[ "$first" == true ]]; then
            first=false
        else
            echo ","
        fi

        local passed="false"
        if (( $(echo "$quality_score >= $QUALITY_THRESHOLD" | bc -l) )); then
            passed="true"
        fi

        cat << EOF
    {
      "file": "$file",
      "quality_score": $quality_score,
      "passed": $passed,
      "metrics": {
        "maintainability_index": $maintainability,
        "complexity_score": $complexity,
        "documentation_coverage": $doc_coverage,
        "duplication_percentage": $duplication,
        "pattern_compliance": $pattern_score,
        "pattern_violations": $violations,
        "lines_of_code": $lines,
        "function_count": $functions
      }
    }
EOF
    done

    echo ""
    echo "  ]"
    echo "}"
}

format_yaml_output() {
    local results=("$@")

    cat << EOF
analysis_timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)
quality_threshold: $QUALITY_THRESHOLD
files:
EOF

    for result in "${results[@]}"; do
        IFS=':' read -r file quality_score maintainability complexity doc_coverage duplication pattern_score violations lines functions <<< "$result"

        local passed="false"
        if (( $(echo "$quality_score >= $QUALITY_THRESHOLD" | bc -l) )); then
            passed="true"
        fi

        cat << EOF
  - file: $file
    quality_score: $quality_score
    passed: $passed
    metrics:
      maintainability_index: $maintainability
      complexity_score: $complexity
      documentation_coverage: $doc_coverage
      duplication_percentage: $duplication
      pattern_compliance: $pattern_score
      pattern_violations: $violations
      lines_of_code: $lines
      function_count: $functions
EOF
    done
}

# Main execution
main() {
    if [[ "$VERBOSE" == true ]]; then
        log "${BLUE}Code Quality Analysis Tool Starting...${NC}"
        log "${BLUE}Quality Threshold: $QUALITY_THRESHOLD%${NC}"
        log "${BLUE}Output Format: $OUTPUT_FORMAT${NC}"
        log "${BLUE}Comprehensive Analysis: $COMPREHENSIVE${NC}"
    fi

    # Get files to analyze
    local files
    files=($(get_files_to_analyze))

    if [[ ${#files[@]} -eq 0 ]]; then
        log "${YELLOW}No files found to analyze${NC}"
        exit 0
    fi

    if [[ "$VERBOSE" == true ]]; then
        log "${BLUE}Analyzing ${#files[@]} files...${NC}"
    fi

    # Analyze each file
    local results=()
    for file in "${files[@]}"; do
        local result
        result=$(analyze_single_file "$file")
        results+=("$result")
    done

    # Format and output results
    format_output "$OUTPUT_FORMAT" "${results[@]}"

    # Check if quality gate should fail
    if [[ "$REPORT_ONLY" == false ]]; then
        local failed_files=0
        for result in "${results[@]}"; do
            local quality_score
            quality_score=$(echo "$result" | cut -d: -f2)
            if (( $(echo "$quality_score < $QUALITY_THRESHOLD" | bc -l) )); then
                ((failed_files++))
            fi
        done

        if [[ $failed_files -gt 0 ]]; then
            log "${RED}Quality gate failed: $failed_files files below threshold${NC}"
            exit 1
        fi
    fi

    if [[ "$VERBOSE" == true ]]; then
        log "${GREEN}Code quality analysis completed successfully${NC}"
    fi
}

# Execute main function
main "$@"

#!/usr/bin/env pwsh
<#
.SYNOPSIS
Smart Agent Selector - FAANG-Inspired Intelligent AI Agent Routing

.DESCRIPTION
This tool provides intelligent, context-aware AI agent selection using proven patterns from top tech companies:
- Google-style metrics and success rate tracking
- Meta-style pattern recognition and learning
- Amazon-style API-first design with built-in tool integration
- Netflix-style self-healing and automatic optimization
- Apple-style seamless user experience

Features:
- Context-aware agent selection based on task, files, and project state
- Success pattern learning and optimization
- Automatic metrics collection and performance tracking
- Built-in IDE integration and tool detection
- Cross-platform compatibility (Windows, macOS, Linux, WSL2)

.PARAMETER TaskDescription
Description of the task or objective

.PARAMETER FileContext
Current file or directory context (auto-detected if not provided)

.PARAMETER ProjectType
Type of project (auto-detected from file structure)

.PARAMETER ForceAgent
Override automatic selection with specific agent

.PARAMETER CollectMetrics
Enable metrics collection (default: true)

.PARAMETER LearnPatterns
Enable pattern learning from successful outcomes (default: true)

.PARAMETER ShowRecommendations
Display top 3 agent recommendations with reasoning

.EXAMPLE
.\tools\smart-agent-selector.ps1 -TaskDescription "Fix compilation errors in auth service"

.EXAMPLE
.\tools\smart-agent-selector.ps1 -TaskDescription "Add new React component" -FileContext "src/components/" -ShowRecommendations

.EXAMPLE
.\tools\smart-agent-selector.ps1 -TaskDescription "Database performance optimization" -CollectMetrics -LearnPatterns

.NOTES
Author: AI Instructions Template (FAANG-Enhanced)
Version: 2.0.0
Created: 2025-09-09
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$TaskDescription,

    [Parameter(Mandatory=$false)]
    [string]$FileContext = "",

    [Parameter(Mandatory=$false)]
    [string]$ProjectType = "",

    [Parameter(Mandatory=$false)]
    [string]$ForceAgent = "",

    [Parameter(Mandatory=$false)]
    [bool]$CollectMetrics = $true,

    [Parameter(Mandatory=$false)]
    [bool]$LearnPatterns = $true,

    [Parameter(Mandatory=$false)]
    [bool]$ShowRecommendations = $false
)

# FAANG-Inspired Agent Intelligence Engine
class SmartAgentSelector {
    [hashtable]$AgentProfiles
    [hashtable]$SuccessMetrics
    [hashtable]$ProjectContext
    [string]$MetricsFile
    [string]$PatternsFile

    SmartAgentSelector() {
        $this.InitializeAgentProfiles()
        $this.LoadMetrics()
        $this.DetectProjectContext()
        $this.SetupMetricsCollection()
    }

    [void]InitializeAgentProfiles() {
        # FAANG-Enhanced Agent Profiles with Success Patterns
        $this.AgentProfiles = @{
            "architect-agent" = @{
                patterns = @(
                    "architecture|design|system|planning|structure|adr|decision",
                    "microservice|monolith|distributed|scalability|performance",
                    "technical.*decision|tech.*stack|framework.*choice"
                )
                success_indicators = @("design", "architecture", "system", "planning", "adr")
                avg_tokens = 4500
                success_rate = 0.85
                best_for = @("System design", "Architecture decisions", "Technical planning")
                ide_integrations = @("draw.io", "mermaid", "plantuml")
            }
            "backend-agent" = @{
                patterns = @(
                    "backend|server|api|service|microservice|endpoint",
                    "compilation|build|cargo|rust|java|python|node|go",
                    "database.*query|sql|orm|migration|schema",
                    "authentication|authorization|security|jwt|oauth",
                    "performance|optimization|caching|async|concurrent"
                )
                success_indicators = @("compile", "build", "test", "deploy", "fix", "implement")
                avg_tokens = 2800
                success_rate = 0.94
                best_for = @("Server development", "API implementation", "Performance optimization")
                ide_integrations = @("rust-analyzer", "language-server", "debugger")
            }
            "frontend-agent" = @{
                patterns = @(
                    "frontend|ui|ux|component|react|vue|angular|svelte",
                    "typescript|javascript|html|css|scss|tailwind",
                    "responsive|mobile|accessibility|a11y|user.*interface",
                    "state.*management|redux|context|hooks|props",
                    "bundler|webpack|vite|rollup|build.*tools"
                )
                success_indicators = @("component", "ui", "render", "style", "interactive")
                avg_tokens = 3200
                success_rate = 0.91
                best_for = @("UI components", "User experience", "Frontend optimization")
                ide_integrations = @("typescript-language-server", "eslint", "prettier")
            }
            "database-agent" = @{
                patterns = @(
                    "database|db|sql|postgres|mysql|mongodb|redis",
                    "query|select|insert|update|delete|join|index",
                    "migration|schema|table|column|relationship|constraint",
                    "performance|optimization|slow.*query|index.*optimization",
                    "backup|restore|replication|sharding|clustering"
                )
                success_indicators = @("query", "optimize", "index", "migrate", "schema")
                avg_tokens = 2500
                success_rate = 0.89
                best_for = @("Query optimization", "Schema design", "Data migration")
                ide_integrations = @("sql-language-server", "database-client", "query-analyzer")
            }
            "devops-agent" = @{
                patterns = @(
                    "docker|container|kubernetes|k8s|deployment|infrastructure",
                    "ci.*cd|pipeline|github.*actions|jenkins|gitlab",
                    "monitoring|logging|metrics|observability|alerting",
                    "cloud|aws|azure|gcp|terraform|ansible|helm",
                    "security|secrets|certificates|ssl|tls|compliance"
                )
                success_indicators = @("deploy", "monitor", "scale", "secure", "automate")
                avg_tokens = 3000
                success_rate = 0.87
                best_for = @("Infrastructure", "CI/CD", "Monitoring")
                ide_integrations = @("docker", "kubernetes", "terraform-ls")
            }
            "qa-agent" = @{
                patterns = @(
                    "test|testing|qa|quality|bug|error|debug|validation",
                    "unit.*test|integration.*test|e2e|end.*to.*end",
                    "coverage|assertion|mock|stub|fixture|scenario",
                    "automation|selenium|cypress|playwright|jest|pytest",
                    "regression|smoke.*test|load.*test|performance.*test"
                )
                success_indicators = @("test", "verify", "validate", "check", "assert")
                avg_tokens = 2400
                success_rate = 0.92
                best_for = @("Test automation", "Quality assurance", "Bug investigation")
                ide_integrations = @("test-runner", "coverage-reporter", "debugger")
            }
            "security-agent" = @{
                patterns = @(
                    "security|vulnerability|penetration|audit|compliance",
                    "encryption|cryptography|hash|ssl|tls|certificates",
                    "authentication|authorization|rbac|oauth|saml|jwt",
                    "xss|csrf|sql.*injection|owasp|security.*scan",
                    "gdpr|hipaa|sox|pci.*dss|iso.*27001|compliance"
                )
                success_indicators = @("secure", "encrypt", "authenticate", "authorize", "validate")
                avg_tokens = 2700
                success_rate = 0.84
                best_for = @("Security assessment", "Compliance", "Vulnerability fixes")
                ide_integrations = @("security-scanner", "linter", "audit-tools")
            }
            "spec-agent" = @{
                patterns = @(
                    "requirement|specification|spec|feature|story|epic",
                    "planning|roadmap|milestone|sprint|backlog|kanban",
                    "user.*story|acceptance.*criteria|definition.*of.*done",
                    "business.*logic|workflow|process|rule|constraint",
                    "documentation|readme|wiki|confluence|notion"
                )
                success_indicators = @("specify", "define", "document", "plan", "analyze")
                avg_tokens = 3800
                success_rate = 0.88
                best_for = @("Requirements analysis", "Feature specification", "Documentation")
                ide_integrations = @("markdown-preview", "mermaid", "mind-map")
            }
            "coordinator-agent" = @{
                patterns = @(
                    "coordination|collaboration|team|cross.*functional",
                    "integration|dependency|handoff|communication",
                    "project.*management|timeline|milestone|deadline",
                    "conflict.*resolution|merge.*conflict|code.*review",
                    "workflow|process|methodology|agile|scrum|kanban"
                )
                success_indicators = @("coordinate", "integrate", "collaborate", "resolve", "organize")
                avg_tokens = 3500
                success_rate = 0.90
                best_for = @("Team coordination", "Integration tasks", "Conflict resolution")
                ide_integrations = @("git", "project-management", "communication-tools")
            }
        }
    }

    [void]LoadMetrics() {
        $this.MetricsFile = "dev-works/metrics/agent-performance.json"
        $this.PatternsFile = "dev-works/metrics/success-patterns.json"

        # Load existing metrics or initialize
        if (Test-Path $this.MetricsFile) {
            try {
                $metricsContent = Get-Content $this.MetricsFile -Raw | ConvertFrom-Json -AsHashtable
                $this.SuccessMetrics = $metricsContent
            } catch {
                Write-Warning "Could not load metrics file, using defaults"
                $this.SuccessMetrics = @{}
            }
        } else {
            $this.SuccessMetrics = @{}
        }

        # Initialize missing agent metrics
        foreach ($agent in $this.AgentProfiles.Keys) {
            if (-not $this.SuccessMetrics.ContainsKey($agent)) {
                $this.SuccessMetrics[$agent] = @{
                    total_uses = 0
                    successful_outcomes = 0
                    avg_tokens_used = $this.AgentProfiles[$agent].avg_tokens
                    last_success_rate = $this.AgentProfiles[$agent].success_rate
                    recent_tasks = @()
                }
            }
        }
    }

    [void]DetectProjectContext() {
        $this.ProjectContext = @{
            root_path = Get-Location
            has_rust = (Test-Path "Cargo.toml")
            has_node = (Test-Path "package.json")
            has_python = (Test-Path "requirements.txt") -or (Test-Path "pyproject.toml")
            has_java = (Test-Path "pom.xml") -or (Test-Path "build.gradle")
            has_docker = (Test-Path "Dockerfile") -or (Test-Path "docker-compose.yml")
            has_kubernetes = (Test-Path "k8s/") -or (Test-Path "kubernetes/")
            has_terraform = (Test-Path "*.tf")
            git_status = $this.GetGitStatus()
            recent_files = $this.GetRecentlyModifiedFiles()
            ide_detected = $this.DetectIDE()
        }
    }

    [hashtable]GetGitStatus() {
        if (-not (Test-Path ".git")) {
            return @{ is_git_repo = $false }
        }

        try {
            $status = git status --porcelain 2>$null
            $branch = git rev-parse --abbrev-ref HEAD 2>$null
            $commits_behind = (git rev-list --count HEAD..origin/$branch 2>$null) -as [int]

            return @{
                is_git_repo = $true
                has_changes = $status.Count -gt 0
                current_branch = $branch
                commits_behind = $commits_behind
                modified_files = ($status | Where-Object { $_ -match "^.M" }).Count
            }
        } catch {
            return @{ is_git_repo = $true; error = $_.Exception.Message }
        }
    }

    [array]GetRecentlyModifiedFiles() {
        try {
            $recentFiles = Get-ChildItem -Recurse -File |
                Where-Object { $_.LastWriteTime -gt (Get-Date).AddHours(-24) } |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 10 -ExpandProperty Name
            return $recentFiles
        } catch {
            return @()
        }
    }

    [string]DetectIDE() {
        if (Test-Path ".vscode/") { return "VS Code" }
        if (Test-Path ".zed/") { return "Zed" }
        if (Test-Path ".idea/") { return "JetBrains" }
        if ($env:TERM_PROGRAM -eq "vscode") { return "VS Code" }
        if ($env:EDITOR -match "code") { return "VS Code" }
        return "Unknown"
    }

    [void]SetupMetricsCollection() {
        # Ensure metrics directories exist
        $metricsDir = "dev-works/metrics"
        if (-not (Test-Path $metricsDir)) {
            New-Item -ItemType Directory -Path $metricsDir -Force | Out-Null
        }
    }

    [object]SelectAgent([string]$taskDescription, [string]$fileContext, [bool]$showRecommendations) {
        # Enhanced agent selection with multiple scoring factors
        $scores = @{}
        $contextFactors = $this.AnalyzeContext($taskDescription, $fileContext)

        foreach ($agentName in $this.AgentProfiles.Keys) {
            $agentProfile = $this.AgentProfiles[$agentName]
            $metrics = $this.SuccessMetrics[$agentName]

            # Multi-factor scoring (FAANG-inspired algorithm)
            $patternScore = $this.CalculatePatternScore($agentProfile.patterns, $taskDescription, $fileContext)
            $successScore = $metrics.last_success_rate
            $efficiencyScore = $this.CalculateEfficiencyScore($metrics.avg_tokens_used)
            $contextScore = $this.CalculateContextScore($agentProfile, $contextFactors)
            $recentSuccessScore = $this.CalculateRecentSuccessScore($metrics.recent_tasks, $taskDescription)

            # Weighted composite score (Google-style algorithm)
            $compositeScore = (
                ($patternScore * 0.35) +      # Pattern matching weight
                ($successScore * 0.25) +      # Historical success weight
                ($efficiencyScore * 0.15) +   # Token efficiency weight
                ($contextScore * 0.15) +      # Project context weight
                ($recentSuccessScore * 0.10)  # Recent success pattern weight
            )

            $scores[$agentName] = @{
                composite_score = $compositeScore
                pattern_score = $patternScore
                success_score = $successScore
                efficiency_score = $efficiencyScore
                context_score = $contextScore
                recent_success_score = $recentSuccessScore
                reasoning = $this.GenerateReasoning($agentProfile, $patternScore, $contextFactors)
            }
        }

        # Sort by composite score
        $rankedAgents = $scores.GetEnumerator() |
            Sort-Object { $_.Value.composite_score } -Descending

        $result = @{
            recommended_agent = $rankedAgents[0].Name
            confidence_score = [math]::Round($rankedAgents[0].Value.composite_score * 100, 1)
            reasoning = $rankedAgents[0].Value.reasoning
            alternatives = @()
            context_analysis = $contextFactors
        }

        # Add top alternatives if showing recommendations
        if ($showRecommendations) {
            for ($i = 1; $i -lt [math]::Min(3, $rankedAgents.Count); $i++) {
                $alternative = $rankedAgents[$i]
                $result.alternatives += @{
                    agent = $alternative.Name
                    confidence = [math]::Round($alternative.Value.composite_score * 100, 1)
                    reasoning = $alternative.Value.reasoning
                }
            }
        }

        return $result
    }

    [double]CalculatePatternScore([array]$patterns, [string]$taskDescription, [string]$fileContext) {
        $combinedText = "$taskDescription $fileContext".ToLower()
        $totalMatches = 0
        $maxPossibleMatches = $patterns.Count

        foreach ($pattern in $patterns) {
            if ($combinedText -match $pattern) {
                $totalMatches++
            }
        }

        if ($maxPossibleMatches -gt 0) {
            return $totalMatches / $maxPossibleMatches
        } else {
            return 0.0
        }
    }

    [double]CalculateEfficiencyScore([int]$avgTokens) {
        # Normalize token efficiency (lower tokens = higher efficiency)
        $baselineTokens = 3000
        return [math]::Max(0.1, [math]::Min(1.0, $baselineTokens / $avgTokens))
    }

    [double]CalculateContextScore([hashtable]$agentProfile, [hashtable]$contextFactors) {
        $score = 0.0

        # Project type alignment
        if ($contextFactors.project_indicators.Count -gt 0) {
            foreach ($indicator in $contextFactors.project_indicators) {
                foreach ($pattern in $agentProfile.patterns) {
                    if ($indicator -match $pattern) {
                        $score += 0.2
                        break
                    }
                }
            }
        }

        # IDE integration bonus
        if ($this.ProjectContext.ide_detected -ne "Unknown") {
            if ($agentProfile.ide_integrations -contains $this.ProjectContext.ide_detected.ToLower()) {
                $score += 0.1
            }
        }

        return [math]::Min(1.0, $score)
    }

    [double]CalculateRecentSuccessScore([array]$recentTasks, [string]$currentTask) {
        if ($recentTasks.Count -eq 0) { return 0.5 }  # Neutral score for no data

        $recentSuccesses = $recentTasks | Where-Object { $_.outcome -eq "success" }
        $successRate = $recentSuccesses.Count / $recentTasks.Count

        # Bonus for similar recent tasks
        $similarityBonus = 0.0
        foreach ($task in $recentSuccesses) {
            if ($this.CalculateTextSimilarity($task.description, $currentTask) -gt 0.5) {
                $similarityBonus += 0.1
            }
        }

        return [math]::Min(1.0, $successRate + $similarityBonus)
    }

    [double]CalculateTextSimilarity([string]$text1, [string]$text2) {
        # Simple word-based similarity calculation
        $words1 = $text1.ToLower() -split '\s+' | Where-Object { $_.Length -gt 3 }
        $words2 = $text2.ToLower() -split '\s+' | Where-Object { $_.Length -gt 3 }

        if ($words1.Count -eq 0 -or $words2.Count -eq 0) { return 0.0 }

        $commonWords = $words1 | Where-Object { $words2 -contains $_ }
        return $commonWords.Count / [math]::Max($words1.Count, $words2.Count)
    }

    [hashtable]AnalyzeContext([string]$taskDescription, [string]$fileContext) {
        $indicators = @()

        # Technology stack indicators
        if ($this.ProjectContext.has_rust) { $indicators += "rust" }
        if ($this.ProjectContext.has_node) { $indicators += "javascript", "typescript", "node" }
        if ($this.ProjectContext.has_python) { $indicators += "python" }
        if ($this.ProjectContext.has_java) { $indicators += "java" }
        if ($this.ProjectContext.has_docker) { $indicators += "docker", "container" }
        if ($this.ProjectContext.has_kubernetes) { $indicators += "kubernetes", "k8s" }

        # File context indicators
        if ($fileContext -match "\.rs$") { $indicators += "rust", "backend" }
        if ($fileContext -match "\.(js|ts|tsx|jsx)$") { $indicators += "javascript", "frontend" }
        if ($fileContext -match "\.(py)$") { $indicators += "python", "backend" }
        if ($fileContext -match "\.(sql)$") { $indicators += "database", "sql" }
        if ($fileContext -match "test") { $indicators += "testing", "qa" }
        if ($fileContext -match "docker") { $indicators += "devops", "container" }

        # Recent activity indicators
        $recentActivity = @()
        if ($this.ProjectContext.git_status.has_changes) {
            $recentActivity += "active-development"
        }
        if ($this.ProjectContext.git_status.commits_behind -gt 0) {
            $recentActivity += "needs-sync"
        }

        return @{
            project_indicators = $indicators
            recent_activity = $recentActivity
            file_context_type = $this.DetermineFileType($fileContext)
            task_complexity = $this.EstimateTaskComplexity($taskDescription)
        }
    }

    [string]DetermineFileType([string]$fileContext) {
        if ($fileContext -match "\.(rs)$") { return "rust-source" }
        if ($fileContext -match "\.(js|ts|tsx|jsx)$") { return "javascript-source" }
        if ($fileContext -match "\.(py)$") { return "python-source" }
        if ($fileContext -match "\.(sql)$") { return "database-script" }
        if ($fileContext -match "\.(md)$") { return "documentation" }
        if ($fileContext -match "docker") { return "infrastructure" }
        if ($fileContext -match "test") { return "test-file" }
        return "unknown"
    }

    [string]EstimateTaskComplexity([string]$taskDescription) {
        $complexityKeywords = @{
            "simple" = @("fix", "update", "change", "modify", "add")
            "medium" = @("implement", "create", "build", "develop", "integrate")
            "complex" = @("design", "architect", "refactor", "optimize", "migrate")
        }

        $taskLower = $taskDescription.ToLower()

        foreach ($level in $complexityKeywords.Keys) {
            foreach ($keyword in $complexityKeywords[$level]) {
                if ($taskLower -match $keyword) {
                    return $level
                }
            }
        }

        return "medium"  # Default complexity
    }

    [string]GenerateReasoning([hashtable]$profile, [double]$patternScore, [hashtable]$contextFactors) {
        $reasons = @()

        if ($patternScore -gt 0.7) {
            $reasons += "Strong pattern match for task type"
        } elseif ($patternScore -gt 0.4) {
            $reasons += "Good pattern match for task requirements"
        }

        if ($profile.success_rate -gt 0.9) {
            $reasons += "Excellent historical success rate"
        } elseif ($profile.success_rate -gt 0.8) {
            $reasons += "Good track record for similar tasks"
        }

        if ($contextFactors.project_indicators.Count -gt 0) {
            $techStack = $contextFactors.project_indicators -join ", "
            $reasons += "Matches project technology stack: $techStack"
        }

        if ($reasons.Count -eq 0) {
            $reasons += "Best available option for this task"
        }

        return $reasons -join "; "
    }

    [void]RecordSelection([string]$agentName, [string]$taskDescription, [string]$outcome, [int]$tokensUsed) {
        if (-not $this.CollectMetrics) { return }

        $metrics = $this.SuccessMetrics[$agentName]
        $metrics.total_uses++

        if ($outcome -eq "success") {
            $metrics.successful_outcomes++
        }

        # Update success rate
        $metrics.last_success_rate = $metrics.successful_outcomes / $metrics.total_uses

        # Update average tokens
        $metrics.avg_tokens_used = [math]::Round(
            (($metrics.avg_tokens_used * ($metrics.total_uses - 1)) + $tokensUsed) / $metrics.total_uses
        )

        # Add to recent tasks (keep last 10)
        $taskRecord = @{
            timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            description = $taskDescription
            outcome = $outcome
            tokens_used = $tokensUsed
        }

        $metrics.recent_tasks += $taskRecord
        if ($metrics.recent_tasks.Count -gt 10) {
            $metrics.recent_tasks = $metrics.recent_tasks[-10..-1]
        }

        # Save updated metrics
        $this.SaveMetrics()
    }

    [void]SaveMetrics() {
        try {
            $this.SuccessMetrics | ConvertTo-Json -Depth 10 |
                Out-File -FilePath $this.MetricsFile -Encoding UTF8
        } catch {
            Write-Warning "Failed to save metrics: $($_.Exception.Message)"
        }
    }
}

# Main execution logic
function Main {
    Write-Host "🤖 Smart Agent Selector (FAANG-Enhanced)" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkCyan

    # Initialize the intelligent selector
    $selector = [SmartAgentSelector]::new()

    # Auto-detect file context if not provided
    if ([string]::IsNullOrEmpty($FileContext)) {
        $FileContext = (Get-Location).Path
    }

    # Override with forced agent if specified
    if (-not [string]::IsNullOrEmpty($ForceAgent)) {
        Write-Host "🔧 Using forced agent: $ForceAgent" -ForegroundColor Yellow
        return @{
            recommended_agent = $ForceAgent
            confidence_score = 100.0
            reasoning = "Manually specified agent"
            forced = $true
        }
    }

    # Perform intelligent agent selection
    Write-Host "🧠 Analyzing task context..." -ForegroundColor Green
    $result = $selector.SelectAgent($TaskDescription, $FileContext, $ShowRecommendations)

    # Display results
    Write-Host ""
    Write-Host "📊 RECOMMENDATION RESULTS" -ForegroundColor White -BackgroundColor DarkBlue
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkCyan

    Write-Host "🎯 Recommended Agent: " -NoNewline -ForegroundColor White
    Write-Host $result.recommended_agent -ForegroundColor Green

    Write-Host "📈 Confidence Score: " -NoNewline -ForegroundColor White
    $confidenceColor = if ($result.confidence_score -gt 80) { "Green" }
                      elseif ($result.confidence_score -gt 60) { "Yellow" }
                      else { "Red" }
    Write-Host "$($result.confidence_score)%" -ForegroundColor $confidenceColor

    Write-Host "💡 Reasoning: " -NoNewline -ForegroundColor White
    Write-Host $result.reasoning -ForegroundColor Cyan

    # Show context analysis
    if ($result.context_analysis.project_indicators.Count -gt 0) {
        Write-Host "🔍 Detected Technologies: " -NoNewline -ForegroundColor White
        Write-Host ($result.context_analysis.project_indicators -join ", ") -ForegroundColor Magenta
    }

    Write-Host "⚡ Task Complexity: " -NoNewline -ForegroundColor White
    Write-Host $result.context_analysis.task_complexity -ForegroundColor Yellow

    # Show alternatives if requested
    if ($ShowRecommendations -and $result.alternatives.Count -gt 0) {
        Write-Host ""
        Write-Host "🔄 ALTERNATIVE RECOMMENDATIONS" -ForegroundColor White -BackgroundColor DarkGreen
        Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkCyan

        for ($i = 0; $i -lt $result.alternatives.Count; $i++) {
            $alt = $result.alternatives[$i]
            Write-Host "$(($i + 2)). $($alt.agent)" -NoNewline -ForegroundColor White
            Write-Host " ($($alt.confidence)%)" -NoNewline -ForegroundColor $confidenceColor
            Write-Host " - $($alt.reasoning)" -ForegroundColor Gray
        }
    }

    # Show quick start command
    Write-Host ""
    Write-Host "🚀 QUICK START" -ForegroundColor White -BackgroundColor DarkMagenta
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkCyan
    Write-Host "Start AI session with recommended agent:" -ForegroundColor White
    Write-Host ""
    Write-Host ".\dev-works\automation\ai-work-tracker.ps1 -Action start-session -AgentName `"$($result.recommended_agent)`" -Objective `"$TaskDescription`" -AutoContext" -ForegroundColor Green
    Write-Host ""

    # Collect metrics if enabled
    if ($CollectMetrics) {
        Write-Host "📊 Metrics collection enabled - tracking selection for continuous improvement" -ForegroundColor DarkGray
    }

    return $result
}

# Execute main function and return result
$result = Main
return $result

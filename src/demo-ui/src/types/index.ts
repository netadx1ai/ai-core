// AI-CORE Demo UI TypeScript Types
// Comprehensive type definitions for the multi-MCP workflow system

export interface McpService {
  name: string;
  url: string;
  port: number;
  status: 'active' | 'inactive' | 'error' | 'starting';
  capabilities: string[];
  lastHealthCheck: string;
  responseTime?: number;
}

export interface WorkflowStep {
  stepId: string;
  stepName: string;
  mcpService: string;
  endpoint: string;
  parameters: Record<string, any>;
  dependsOn: string[];
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';
  result?: any;
  error?: string;
  processingTimeMs?: number;
  startedAt?: string;
  completedAt?: string;
  progress?: number;
}

export interface Workflow {
  workflowId: string;
  workflowType: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  steps: WorkflowStep[];
  results: Record<string, any>;
  createdAt: string;
  updatedAt: string;
  processingTimeMs?: number;
  parameters: Record<string, any>;
  options?: WorkflowOptions;
}

export interface WorkflowOptions {
  timeoutSeconds?: number;
  parallelExecution?: boolean;
  failureStrategy?: 'fail_fast' | 'continue' | 'retry';
  notificationWebhook?: string;
}

export interface ContentGenerationRequest {
  contentType: 'blog_post' | 'social_media_post' | 'email_newsletter';
  topic: string;
  targetAudience?: string;
  tone?: string;
  wordCount?: number;
  includeImages?: boolean;
}

export interface ContentGenerationResponse {
  id: string;
  contentType: string;
  title: string;
  content: string;
  wordCount: number;
  status: string;
  createdAt: string;
  processingTimeMs: number;
  aiModel: string;
}

export interface TextAnalysisRequest {
  text: string;
  analysisType: 'keywords' | 'sentiment' | 'readability' | 'grammar' | 'summary';
  language?: string;
  options?: AnalysisOptions;
}

export interface AnalysisOptions {
  maxKeywords?: number;
  summaryLength?: 'short' | 'medium' | 'long';
  sentimentDetail?: boolean;
  readabilityMetrics?: boolean;
}

export interface TextAnalysisResponse {
  id: string;
  analysisType: string;
  originalTextStats: TextStats;
  results: AnalysisResults;
  processingTimeMs: number;
  aiModel: string;
  createdAt: string;
}

export interface TextStats {
  characterCount: number;
  wordCount: number;
  sentenceCount: number;
  paragraphCount: number;
  avgWordsPerSentence: number;
}

export interface AnalysisResults {
  keywords?: KeywordAnalysis;
  sentiment?: SentimentAnalysis;
  readability?: ReadabilityAnalysis;
  grammar?: GrammarAnalysis;
  summary?: SummaryAnalysis;
}

export interface KeywordAnalysis {
  keywords: Keyword[];
  phrases: KeyPhrase[];
  topics: string[];
  confidenceScore: number;
}

export interface Keyword {
  word: string;
  frequency: number;
  relevanceScore: number;
  category?: string;
}

export interface KeyPhrase {
  phrase: string;
  frequency: number;
  importanceScore: number;
}

export interface SentimentAnalysis {
  overallSentiment: 'positive' | 'negative' | 'neutral';
  confidenceScore: number;
  emotionalTone: EmotionalTone[];
  sentimentBySentence?: SentenceSentiment[];
}

export interface EmotionalTone {
  emotion: string;
  intensity: number;
}

export interface SentenceSentiment {
  sentence: string;
  sentiment: string;
  confidence: number;
}

export interface ReadabilityAnalysis {
  readingLevel: string;
  complexityScore: number;
  avgSentenceLength: number;
  difficultWordsPercentage: number;
  suggestions: string[];
}

export interface GrammarAnalysis {
  grammarScore: number;
  issuesFound: GrammarIssue[];
  suggestions: string[];
  correctedText?: string;
}

export interface GrammarIssue {
  issueType: string;
  description: string;
  position: number;
  severity: 'low' | 'medium' | 'high';
  suggestion: string;
}

export interface SummaryAnalysis {
  summary: string;
  keyPoints: string[];
  summaryType: string;
  compressionRatio: number;
  originalLength: number;
  summaryLength: number;
}

export interface ImageGenerationRequest {
  prompt: string;
  style?: 'realistic' | 'artistic' | 'cartoon' | 'abstract' | 'photographic' | 'digital_art';
  size?: '256x256' | '512x512' | '1024x1024' | '1792x1024';
  quality?: 'standard' | 'hd';
  count?: number;
}

export interface ImageGenerationResponse {
  id: string;
  prompt: string;
  images: GeneratedImage[];
  processingTimeMs: number;
  aiModel: string;
  createdAt: string;
  status: string;
}

export interface GeneratedImage {
  imageId: string;
  url: string;
  size: string;
  style: string;
  base64Data?: string;
}

export interface WorkflowTemplate {
  id: string;
  name: string;
  type: string;
  description: string;
  requiredParameters: string[];
  optionalParameters: string[];
  estimatedDuration: number;
  steps: Omit<WorkflowStep, 'stepId' | 'status' | 'result' | 'error' | 'processingTimeMs' | 'startedAt' | 'completedAt'>[];
}

export interface ServiceHealth {
  status: string;
  service: string;
  timestamp: string;
  geminiAvailable?: boolean;
  registeredMcps?: number;
  activeWorkflows?: number;
  supportedWorkflowTypes?: string[];
  supportedLanguages?: string[];
}

export interface DemoSession {
  sessionId: string;
  startedAt: string;
  workflows: Workflow[];
  totalProcessingTime: number;
  successfulWorkflows: number;
  failedWorkflows: number;
  generatedContent: ContentGenerationResponse[];
  analysisResults: TextAnalysisResponse[];
  generatedImages: ImageGenerationResponse[];
}

export interface NotificationMessage {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  title: string;
  message: string;
  timestamp: string;
  autoHide?: boolean;
  duration?: number;
}

export interface ApiError {
  message: string;
  status?: number;
  code?: string;
  details?: Record<string, any>;
}

export interface LiveMetrics {
  timestamp: string;
  activeWorkflows: number;
  completedWorkflows: number;
  totalProcessingTime: number;
  averageResponseTime: number;
  successRate: number;
  servicesOnline: number;
  totalServices: number;
}

export interface ExportData {
  sessionId: string;
  exportedAt: string;
  workflows: Workflow[];
  content: ContentGenerationResponse[];
  analysis: TextAnalysisResponse[];
  images: ImageGenerationResponse[];
  metrics: LiveMetrics;
}

// Component Props Types
export interface WorkflowCardProps {
  workflow: Workflow;
  onViewDetails: (workflowId: string) => void;
  onCancel?: (workflowId: string) => void;
}

export interface ServiceStatusProps {
  services: McpService[];
  onRefresh: () => void;
}

export interface WorkflowStepProps {
  step: WorkflowStep;
  isActive: boolean;
  isCompleted: boolean;
}

export interface ContentPreviewProps {
  content: ContentGenerationResponse;
  onExport?: (format: 'markdown' | 'html' | 'pdf') => void;
}

export interface AnalysisVisualizationProps {
  analysis: TextAnalysisResponse;
  type: 'keywords' | 'sentiment' | 'readability';
}

export interface MetricsDashboardProps {
  metrics: LiveMetrics[];
  timeRange: '1h' | '6h' | '24h' | '7d';
}

// Store Types (Zustand)
export interface AppStore {
  // Services
  services: McpService[];
  servicesLoading: boolean;

  // Workflows
  workflows: Workflow[];
  activeWorkflow: Workflow | null;
  workflowsLoading: boolean;

  // Session
  currentSession: DemoSession | null;

  // UI State
  notifications: NotificationMessage[];
  sidebarOpen: boolean;

  // Actions
  setServices: (services: McpService[]) => void;
  addWorkflow: (workflow: Workflow) => void;
  updateWorkflow: (workflowId: string, updates: Partial<Workflow>) => void;
  setActiveWorkflow: (workflow: Workflow | null) => void;
  addNotification: (notification: NotificationMessage) => void;
  removeNotification: (id: string) => void;
  toggleSidebar: () => void;
  startSession: () => void;
  endSession: () => void;
}

// API Response Types
export interface ApiResponse<T = any> {
  data: T;
  success: boolean;
  message?: string;
  timestamp: string;
}

export interface PaginatedResponse<T = any> {
  data: T[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    totalPages: number;
  };
}

// Configuration Types
export interface AppConfig {
  apiBaseUrl: string;
  services: {
    contentMcp: string;
    textProcessingMcp: string;
    imageGenerationMcp: string;
    orchestrator: string;
  };
  features: {
    realTimeUpdates: boolean;
    exportFeatures: boolean;
    analytics: boolean;
  };
  ui: {
    theme: 'light' | 'dark';
    autoRefreshInterval: number;
    maxNotifications: number;
  };
}

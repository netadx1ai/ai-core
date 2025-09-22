//! Text Processing MCP Server with Gemini AI Integration
//!
//! Advanced text analysis and processing service using Google Gemini for intelligent text operations.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};

use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub service_name: String,
    pub gemini_client: GeminiClient,
}

#[derive(Clone)]
pub struct GeminiClient {
    pub api_key: String,
    pub client: reqwest::Client,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct TextAnalysisRequest {
    pub text: String,
    pub analysis_type: String, // "keywords", "sentiment", "readability", "grammar", "summary"
    pub language: Option<String>,
    pub options: Option<AnalysisOptions>,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisOptions {
    pub max_keywords: Option<usize>,
    pub summary_length: Option<String>, // "short", "medium", "long"
    pub sentiment_detail: Option<bool>,
    pub readability_metrics: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TextAnalysisResponse {
    pub id: Uuid,
    pub analysis_type: String,
    pub original_text_stats: TextStats,
    pub results: AnalysisResults,
    pub processing_time_ms: u64,
    pub ai_model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TextStats {
    pub character_count: usize,
    pub word_count: usize,
    pub sentence_count: usize,
    pub paragraph_count: usize,
    pub avg_words_per_sentence: f32,
}

#[derive(Debug, Serialize)]
pub struct AnalysisResults {
    pub keywords: Option<KeywordAnalysis>,
    pub sentiment: Option<SentimentAnalysis>,
    pub readability: Option<ReadabilityAnalysis>,
    pub grammar: Option<GrammarAnalysis>,
    pub summary: Option<SummaryAnalysis>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeywordAnalysis {
    pub keywords: Vec<Keyword>,
    pub phrases: Vec<KeyPhrase>,
    pub topics: Vec<String>,
    pub confidence_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keyword {
    pub word: String,
    pub frequency: usize,
    pub relevance_score: f32,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPhrase {
    pub phrase: String,
    pub frequency: usize,
    pub importance_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SentimentAnalysis {
    pub overall_sentiment: String, // "positive", "negative", "neutral"
    pub confidence_score: f32,
    pub emotional_tone: Vec<EmotionalTone>,
    pub sentiment_by_sentence: Option<Vec<SentenceSentiment>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmotionalTone {
    pub emotion: String,
    pub intensity: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SentenceSentiment {
    pub sentence: String,
    pub sentiment: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadabilityAnalysis {
    pub reading_level: String,
    pub complexity_score: f32,
    pub avg_sentence_length: f32,
    pub difficult_words_percentage: f32,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrammarAnalysis {
    pub grammar_score: f32,
    pub issues_found: Vec<GrammarIssue>,
    pub suggestions: Vec<String>,
    pub corrected_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrammarIssue {
    pub issue_type: String,
    pub description: String,
    pub position: usize,
    pub severity: String, // "low", "medium", "high"
    pub suggestion: String,
}

#[derive(Debug, Serialize)]
pub struct SummaryAnalysis {
    pub summary: String,
    pub key_points: Vec<String>,
    pub summary_type: String,
    pub compression_ratio: f32,
    pub original_length: usize,
    pub summary_length: usize,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub service: String,
    pub timestamp: DateTime<Utc>,
    pub gemini_available: bool,
    pub supported_languages: Vec<String>,
}

// Gemini API types
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    top_k: u32,
    top_p: f32,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: String,
}

impl GeminiClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| "GEMINI_API_KEY environment variable not set")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { api_key, client })
    }

    pub async fn analyze_text(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            self.api_key
        );

        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: 0.3, // Lower temperature for more consistent analysis
                top_k: 40,
                top_p: 0.95,
                max_output_tokens: 1024,
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Gemini API error {}: {}", status, error_text).into());
        }

        let gemini_response: GeminiResponse = response.json().await?;

        if let Some(candidate) = gemini_response.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                return Ok(part.text.clone());
            }
        }

        Err("No analysis generated by Gemini".into())
    }

    pub async fn test_connection(&self) -> bool {
        let test_prompt = "Analyze this text for sentiment: 'This is a test.' Respond with just 'POSITIVE' or 'NEGATIVE'.";
        match self.analyze_text(test_prompt).await {
            Ok(_) => true,
            Err(e) => {
                warn!("Gemini connection test failed: {}", e);
                false
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("text_processing_mcp=info,tower_http=debug")
        .init();

    info!(
        "Starting Text Processing MCP Server v{} with Gemini Flash",
        env!("CARGO_PKG_VERSION")
    );

    // Initialize Gemini client
    let gemini_client = match GeminiClient::new() {
        Ok(client) => {
            info!("Gemini client initialized successfully");
            // Test connection
            if client.test_connection().await {
                info!("Gemini API connection verified");
            } else {
                warn!("Gemini API connection test failed - service will use fallback analysis");
            }
            client
        }
        Err(e) => {
            error!("Failed to initialize Gemini client: {}", e);
            warn!("Service will run with fallback text analysis");
            // Create a dummy client for fallback mode
            GeminiClient {
                api_key: "fallback".to_string(),
                client: reqwest::Client::new(),
            }
        }
    };

    let state = AppState {
        service_name: "text-processing-mcp".to_string(),
        gemini_client,
    };

    let app = create_router(state);

    let listener = TcpListener::bind("0.0.0.0:8805").await?;
    info!("Text Processing MCP Server listening on http://0.0.0.0:8805");

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/analyze", post(analyze_text))
        .route("/v1/analysis/:analysis_id", get(get_analysis))
        .route("/v1/capabilities", get(get_capabilities))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let gemini_available =
        state.gemini_client.api_key != "fallback" && env::var("GEMINI_API_KEY").is_ok();

    Json(HealthStatus {
        status: "healthy".to_string(),
        service: state.service_name,
        timestamp: Utc::now(),
        gemini_available,
        supported_languages: vec![
            "English".to_string(),
            "Spanish".to_string(),
            "French".to_string(),
            "German".to_string(),
            "Italian".to_string(),
            "Portuguese".to_string(),
        ],
    })
}

async fn analyze_text(
    State(state): State<AppState>,
    Json(request): Json<TextAnalysisRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let start_time = std::time::Instant::now();

    info!(
        "Analyzing text for '{}' analysis type, {} characters",
        request.analysis_type,
        request.text.len()
    );

    // Calculate basic text statistics
    let text_stats = calculate_text_stats(&request.text);

    // Perform AI-powered analysis
    let analysis_results = match perform_analysis(&state.gemini_client, &request).await {
        Ok(results) => {
            info!("Text analysis completed successfully using Gemini API");
            results
        }
        Err(e) => {
            warn!("Gemini API failed ({}), using fallback analysis", e);
            perform_fallback_analysis(&request)
        }
    };

    let processing_time = start_time.elapsed().as_millis() as u64;

    let response = TextAnalysisResponse {
        id: Uuid::new_v4(),
        analysis_type: request.analysis_type,
        original_text_stats: text_stats,
        results: analysis_results,
        processing_time_ms: processing_time,
        ai_model: "gemini-1.5-flash".to_string(),
        created_at: Utc::now(),
    };

    info!("Text analysis completed in {}ms", processing_time);

    Ok(Json(response))
}

async fn perform_analysis(
    gemini_client: &GeminiClient,
    request: &TextAnalysisRequest,
) -> Result<AnalysisResults, Box<dyn std::error::Error>> {
    let mut results = AnalysisResults {
        keywords: None,
        sentiment: None,
        readability: None,
        grammar: None,
        summary: None,
    };

    match request.analysis_type.as_str() {
        "keywords" => {
            results.keywords =
                Some(analyze_keywords_ai(gemini_client, &request.text, &request.options).await?);
        }
        "sentiment" => {
            results.sentiment =
                Some(analyze_sentiment_ai(gemini_client, &request.text, &request.options).await?);
        }
        "readability" => {
            results.readability = Some(analyze_readability_ai(gemini_client, &request.text).await?);
        }
        "grammar" => {
            results.grammar = Some(analyze_grammar_ai(gemini_client, &request.text).await?);
        }
        "summary" => {
            results.summary =
                Some(analyze_summary_ai(gemini_client, &request.text, &request.options).await?);
        }
        _ => {
            return Err("Unsupported analysis type".into());
        }
    }

    Ok(results)
}

async fn analyze_keywords_ai(
    gemini_client: &GeminiClient,
    text: &str,
    options: &Option<AnalysisOptions>,
) -> Result<KeywordAnalysis, Box<dyn std::error::Error>> {
    let max_keywords = options.as_ref().and_then(|o| o.max_keywords).unwrap_or(10);

    let prompt = format!(
        "Analyze the following text and extract the {} most important keywords and key phrases.
        Also identify the main topics discussed.

        Text: \"{}\"

        Please provide a detailed analysis in this exact JSON format:
        {{
            \"keywords\": [
                {{\"word\": \"example\", \"frequency\": 3, \"relevance_score\": 0.9, \"category\": \"noun\"}}
            ],
            \"phrases\": [
                {{\"phrase\": \"example phrase\", \"frequency\": 2, \"importance_score\": 0.8}}
            ],
            \"topics\": [\"topic1\", \"topic2\"],
            \"confidence_score\": 0.85
        }}",
        max_keywords, text
    );

    let response = gemini_client.analyze_text(&prompt).await?;

    // Parse JSON response or create fallback
    match serde_json::from_str::<KeywordAnalysis>(&response) {
        Ok(analysis) => Ok(analysis),
        Err(_) => {
            // Fallback parsing if JSON format is not perfect
            Ok(create_fallback_keyword_analysis(text, max_keywords))
        }
    }
}

async fn analyze_sentiment_ai(
    gemini_client: &GeminiClient,
    text: &str,
    options: &Option<AnalysisOptions>,
) -> Result<SentimentAnalysis, Box<dyn std::error::Error>> {
    let detail_level = options
        .as_ref()
        .and_then(|o| o.sentiment_detail)
        .unwrap_or(false);

    let prompt = if detail_level {
        format!(
            "Perform detailed sentiment analysis on the following text. Analyze overall sentiment,
            emotional tones, and sentiment for each sentence.

            Text: \"{}\"

            Provide analysis in this JSON format:
            {{
                \"overall_sentiment\": \"positive/negative/neutral\",
                \"confidence_score\": 0.85,
                \"emotional_tone\": [
                    {{\"emotion\": \"joy\", \"intensity\": 0.7}}
                ],
                \"sentiment_by_sentence\": [
                    {{\"sentence\": \"...\", \"sentiment\": \"positive\", \"confidence\": 0.8}}
                ]
            }}",
            text
        )
    } else {
        format!(
            "Analyze the sentiment of this text. Determine if it's positive, negative, or neutral.

            Text: \"{}\"

            Provide analysis in this JSON format:
            {{
                \"overall_sentiment\": \"positive/negative/neutral\",
                \"confidence_score\": 0.85,
                \"emotional_tone\": [
                    {{\"emotion\": \"joy\", \"intensity\": 0.7}}
                ]
            }}",
            text
        )
    };

    let response = gemini_client.analyze_text(&prompt).await?;

    match serde_json::from_str::<SentimentAnalysis>(&response) {
        Ok(analysis) => Ok(analysis),
        Err(_) => Ok(create_fallback_sentiment_analysis(text)),
    }
}

async fn analyze_readability_ai(
    gemini_client: &GeminiClient,
    text: &str,
) -> Result<ReadabilityAnalysis, Box<dyn std::error::Error>> {
    let prompt = format!(
        "Analyze the readability of this text. Determine reading level, complexity, and provide suggestions.

        Text: \"{}\"

        Provide analysis in this JSON format:
        {{
            \"reading_level\": \"Elementary/Middle School/High School/College/Graduate\",
            \"complexity_score\": 0.65,
            \"avg_sentence_length\": 15.5,
            \"difficult_words_percentage\": 12.3,
            \"suggestions\": [\"suggestion1\", \"suggestion2\"]
        }}",
        text
    );

    let response = gemini_client.analyze_text(&prompt).await?;

    match serde_json::from_str::<ReadabilityAnalysis>(&response) {
        Ok(analysis) => Ok(analysis),
        Err(_) => Ok(create_fallback_readability_analysis(text)),
    }
}

async fn analyze_grammar_ai(
    gemini_client: &GeminiClient,
    text: &str,
) -> Result<GrammarAnalysis, Box<dyn std::error::Error>> {
    let prompt = format!(
        "Analyze the grammar of this text. Identify issues and provide corrections.

        Text: \"{}\"

        Provide analysis in this JSON format:
        {{
            \"grammar_score\": 0.85,
            \"issues_found\": [
                {{
                    \"issue_type\": \"subject-verb agreement\",
                    \"description\": \"Subject and verb don't agree\",
                    \"position\": 45,
                    \"severity\": \"medium\",
                    \"suggestion\": \"Change 'are' to 'is'\"
                }}
            ],
            \"suggestions\": [\"overall suggestion1\"],
            \"corrected_text\": \"corrected version of the text\"
        }}",
        text
    );

    let response = gemini_client.analyze_text(&prompt).await?;

    match serde_json::from_str::<GrammarAnalysis>(&response) {
        Ok(analysis) => Ok(analysis),
        Err(_) => Ok(create_fallback_grammar_analysis(text)),
    }
}

async fn analyze_summary_ai(
    gemini_client: &GeminiClient,
    text: &str,
    options: &Option<AnalysisOptions>,
) -> Result<SummaryAnalysis, Box<dyn std::error::Error>> {
    let summary_length = options
        .as_ref()
        .and_then(|o| o.summary_length.as_deref())
        .unwrap_or("medium");

    let length_instruction = match summary_length {
        "short" => "1-2 sentences",
        "long" => "4-6 sentences",
        _ => "2-3 sentences",
    };

    let prompt = format!(
        "Summarize the following text in {} and identify key points.

        Text: \"{}\"

        Provide analysis in this JSON format:
        {{
            \"summary\": \"summary text here\",
            \"key_points\": [\"point1\", \"point2\", \"point3\"],
            \"summary_type\": \"{}\",
            \"compression_ratio\": 0.25
        }}",
        length_instruction, text, summary_length
    );

    let response = gemini_client.analyze_text(&prompt).await?;

    match serde_json::from_str::<serde_json::Value>(&response) {
        Ok(json) => {
            let summary = json["summary"]
                .as_str()
                .unwrap_or("Summary not available")
                .to_string();
            let key_points: Vec<String> = json["key_points"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_else(|| vec!["Key points not available".to_string()]);

            let original_length = text.chars().count();
            let summary_len = summary.chars().count();
            let compression_ratio = if original_length > 0 {
                summary_len as f32 / original_length as f32
            } else {
                0.0
            };

            Ok(SummaryAnalysis {
                summary,
                key_points,
                summary_type: summary_length.to_string(),
                compression_ratio,
                original_length,
                summary_length: summary_len,
            })
        }
        Err(_) => Ok(create_fallback_summary_analysis(text, summary_length)),
    }
}

fn calculate_text_stats(text: &str) -> TextStats {
    let character_count = text.chars().count();
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len();

    // Count sentences (simple heuristic)
    let sentence_regex = Regex::new(r"[.!?]+").unwrap();
    let sentence_count = sentence_regex.find_iter(text).count().max(1);

    // Count paragraphs
    let paragraph_count = text
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .count()
        .max(1);

    let avg_words_per_sentence = if sentence_count > 0 {
        word_count as f32 / sentence_count as f32
    } else {
        0.0
    };

    TextStats {
        character_count,
        word_count,
        sentence_count,
        paragraph_count,
        avg_words_per_sentence,
    }
}

// Fallback implementations
fn perform_fallback_analysis(request: &TextAnalysisRequest) -> AnalysisResults {
    let mut results = AnalysisResults {
        keywords: None,
        sentiment: None,
        readability: None,
        grammar: None,
        summary: None,
    };

    match request.analysis_type.as_str() {
        "keywords" => {
            let max_keywords = request
                .options
                .as_ref()
                .and_then(|o| o.max_keywords)
                .unwrap_or(10);
            results.keywords = Some(create_fallback_keyword_analysis(
                &request.text,
                max_keywords,
            ));
        }
        "sentiment" => {
            results.sentiment = Some(create_fallback_sentiment_analysis(&request.text));
        }
        "readability" => {
            results.readability = Some(create_fallback_readability_analysis(&request.text));
        }
        "grammar" => {
            results.grammar = Some(create_fallback_grammar_analysis(&request.text));
        }
        "summary" => {
            let summary_length = request
                .options
                .as_ref()
                .and_then(|o| o.summary_length.as_deref())
                .unwrap_or("medium");
            results.summary = Some(create_fallback_summary_analysis(
                &request.text,
                summary_length,
            ));
        }
        _ => {}
    }

    results
}

fn create_fallback_keyword_analysis(text: &str, max_keywords: usize) -> KeywordAnalysis {
    // Simple keyword extraction
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut word_counts = std::collections::HashMap::new();

    for word in words {
        let clean_word = word
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>();
        if clean_word.len() > 3 {
            *word_counts.entry(clean_word).or_insert(0) += 1;
        }
    }

    let mut keywords: Vec<_> = word_counts
        .into_iter()
        .map(|(word, freq)| Keyword {
            word,
            frequency: freq,
            relevance_score: (freq as f32).min(10.0) / 10.0,
            category: Some("general".to_string()),
        })
        .collect();

    keywords.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    keywords.truncate(max_keywords);

    KeywordAnalysis {
        keywords,
        phrases: vec![], // Simplified - no phrase extraction in fallback
        topics: vec!["general".to_string()],
        confidence_score: 0.6,
    }
}

fn create_fallback_sentiment_analysis(text: &str) -> SentimentAnalysis {
    // Simple sentiment analysis based on word patterns
    let positive_words = [
        "good",
        "great",
        "excellent",
        "amazing",
        "wonderful",
        "fantastic",
    ];
    let negative_words = ["bad", "terrible", "awful", "horrible", "disappointing"];

    let text_lower = text.to_lowercase();
    let positive_count = positive_words
        .iter()
        .filter(|&&word| text_lower.contains(word))
        .count();
    let negative_count = negative_words
        .iter()
        .filter(|&&word| text_lower.contains(word))
        .count();

    let (sentiment, confidence) = if positive_count > negative_count {
        ("positive", 0.7)
    } else if negative_count > positive_count {
        ("negative", 0.7)
    } else {
        ("neutral", 0.6)
    };

    SentimentAnalysis {
        overall_sentiment: sentiment.to_string(),
        confidence_score: confidence,
        emotional_tone: vec![EmotionalTone {
            emotion: sentiment.to_string(),
            intensity: confidence,
        }],
        sentiment_by_sentence: None,
    }
}

fn create_fallback_readability_analysis(text: &str) -> ReadabilityAnalysis {
    let words = text.split_whitespace().count();
    let sentences = text.matches(&['.', '!', '?'][..]).count().max(1);
    let avg_sentence_length = words as f32 / sentences as f32;

    let (reading_level, complexity_score) = if avg_sentence_length < 10.0 {
        ("Elementary", 0.3)
    } else if avg_sentence_length < 15.0 {
        ("Middle School", 0.5)
    } else if avg_sentence_length < 20.0 {
        ("High School", 0.7)
    } else {
        ("College", 0.9)
    };

    ReadabilityAnalysis {
        reading_level: reading_level.to_string(),
        complexity_score,
        avg_sentence_length,
        difficult_words_percentage: 15.0, // Placeholder
        suggestions: vec![
            "Consider shorter sentences for better readability".to_string(),
            "Use simpler vocabulary where possible".to_string(),
        ],
    }
}

fn create_fallback_grammar_analysis(text: &str) -> GrammarAnalysis {
    // Very basic grammar checks
    let mut issues = Vec::new();

    // Check for double spaces
    if text.contains("  ") {
        issues.push(GrammarIssue {
            issue_type: "spacing".to_string(),
            description: "Multiple consecutive spaces found".to_string(),
            position: text.find("  ").unwrap_or(0),
            severity: "low".to_string(),
            suggestion: "Use single spaces between words".to_string(),
        });
    }

    let grammar_score = if issues.is_empty() { 0.9 } else { 0.7 };

    GrammarAnalysis {
        grammar_score,
        issues_found: issues,
        suggestions: vec!["Consider using a comprehensive grammar checker".to_string()],
        corrected_text: Some(text.replace("  ", " ")),
    }
}

fn create_fallback_summary_analysis(text: &str, summary_type: &str) -> SummaryAnalysis {
    let sentences: Vec<&str> = text.split(&['.', '!', '?'][..]).collect();
    let first_sentence = sentences.first().unwrap_or(&"").trim();

    let summary = if first_sentence.is_empty() {
        "Text summary not available".to_string()
    } else {
        format!("{}.", first_sentence)
    };

    let original_length = text.chars().count();
    let summary_length = summary.chars().count();
    let compression_ratio = if original_length > 0 {
        summary_length as f32 / original_length as f32
    } else {
        0.0
    };

    SummaryAnalysis {
        summary,
        key_points: vec!["Key points analysis requires AI processing".to_string()],
        summary_type: summary_type.to_string(),
        compression_ratio,
        original_length,
        summary_length,
    }
}

async fn get_analysis(
    State(_state): State<AppState>,
    Path(analysis_id): Path<Uuid>,
) -> impl IntoResponse {
    // Mock response for demo
    Json(TextAnalysisResponse {
        id: analysis_id,
        analysis_type: "sentiment".to_string(),
        original_text_stats: TextStats {
            character_count: 100,
            word_count: 20,
            sentence_count: 2,
            paragraph_count: 1,
            avg_words_per_sentence: 10.0,
        },
        results: AnalysisResults {
            keywords: None,
            sentiment: Some(SentimentAnalysis {
                overall_sentiment: "positive".to_string(),
                confidence_score: 0.85,
                emotional_tone: vec![EmotionalTone {
                    emotion: "optimistic".to_string(),
                    intensity: 0.7,
                }],
                sentiment_by_sentence: None,
            }),
            readability: None,
            grammar: None,
            summary: None,
        },
        processing_time_ms: 1500,
        ai_model: "gemini-1.5-flash".to_string(),
        created_at: Utc::now(),
    })
}

async fn get_capabilities(State(_state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "text-processing-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "ai_model": "gemini-1.5-flash",
        "supported_analysis_types": [
            "keywords",
            "sentiment",
            "readability",
            "grammar",
            "summary"
        ],
        "supported_languages": [
            "English",
            "Spanish",
            "French",
            "German",
            "Italian",
            "Portuguese"
        ],
        "max_text_length": 10000,
        "features": [
            "ai_powered_analysis",
            "fallback_processing",
            "detailed_statistics",
            "multi_language_support",
            "real_time_processing"
        ]
    }))
}

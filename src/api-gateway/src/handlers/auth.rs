//! Authentication handlers for login, logout, refresh token, and user profile operations

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use validator::Validate;

use crate::{
    error::{ApiError, Result},
    middleware_layer::auth::{require_user_context, UserContext},
    state::AppState,
};
use ai_core_shared::types::core::{SubscriptionTier, User};

/// Login request payload
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    /// Optional 2FA code
    pub totp_code: Option<String>,

    /// Remember me flag for extended session
    #[serde(default)]
    pub remember_me: bool,
}

/// API key login request payload
#[derive(Debug, Deserialize, Validate)]
pub struct ApiKeyLoginRequest {
    #[validate(length(min = 32, message = "Invalid API key format"))]
    pub api_key: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user: UserProfile,
}

/// Refresh token request
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

/// User profile response
#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub subscription_tier: SubscriptionTier,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub email_verified: bool,
    pub two_factor_enabled: bool,
}

/// Update profile request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: Option<String>,

    #[validate(url(message = "Invalid avatar URL format"))]
    pub avatar_url: Option<String>,

    /// User preferences as JSON object
    pub preferences: Option<serde_json::Value>,
}

/// Change password request
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 8, message = "Current password is required"))]
    pub current_password: String,

    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    pub new_password: String,

    /// Optional 2FA code for additional security
    pub totp_code: Option<String>,
}

/// Logout response
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// POST /auth/login - Authenticate with email/password
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    // Validate input
    payload
        .validate()
        .map_err(|e| ApiError::validation("login", format!("Invalid login request: {}", e)))?;

    info!("Login attempt for email: {}", payload.email);

    // Authenticate user
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    let auth_result = auth_service
        .authenticate_user(
            &payload.email,
            &payload.password,
            payload.totp_code.as_deref(),
        )
        .await?;

    // Check if user exists and password is correct
    let user = match auth_result {
        Some(user) => user,
        None => {
            warn!("Failed login attempt for email: {}", payload.email);
            return Err(ApiError::authentication("Invalid email or password"));
        }
    };

    // Check if account is active
    if !user.is_active {
        return Err(ApiError::authentication("Account is disabled"));
    }

    // Check if email is verified
    if !user.email_verified {
        return Err(ApiError::authentication("Email verification required"));
    }

    // Generate tokens
    let token_expiry = if payload.remember_me {
        30 * 24 * 60 * 60
    } else {
        60 * 60
    }; // 30 days or 1 hour

    let access_token = auth_service
        .generate_access_token(&user, token_expiry)
        .await?;

    let refresh_token = auth_service.generate_refresh_token(&user).await?;

    // Update last login time
    auth_service.update_last_login(&user.id).await?;

    // Record login metrics
    state
        .metrics
        .record_user_login(&user.id, &user.subscription_tier.to_string());

    info!("Successful login for user: {}", user.id);

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        expires_in: token_expiry,
        token_type: "Bearer".to_string(),
        user: UserProfile {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user.name.clone(),
            avatar_url: user.avatar_url.clone(),
            subscription_tier: user.subscription_tier.clone(),
            roles: user.roles.clone(),
            permissions: user.permissions.clone(),
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login_at: Some(chrono::Utc::now()),
            email_verified: user.email_verified,
            two_factor_enabled: user.totp_secret.is_some(),
        },
    }))
}

/// POST /auth/login/api-key - Authenticate with API key
pub async fn login_with_api_key(
    State(state): State<AppState>,
    Json(payload): Json<ApiKeyLoginRequest>,
) -> Result<Json<LoginResponse>> {
    // Validate input
    payload
        .validate()
        .map_err(|e| ApiError::validation("api_key", format!("Invalid API key request: {}", e)))?;

    info!("API key login attempt");

    // Authenticate with API key
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    let user = auth_service
        .authenticate_api_key(&payload.api_key)
        .await?
        .ok_or_else(|| ApiError::authentication("Invalid API key"))?;

    // Check if account is active
    if !user.is_active {
        return Err(ApiError::authentication("Account is disabled"));
    }

    // Generate tokens (longer expiry for API key auth)
    let token_expiry = 24 * 60 * 60; // 24 hours

    let access_token = auth_service
        .generate_access_token(&user, token_expiry)
        .await?;

    let refresh_token = auth_service.generate_refresh_token(&user).await?;

    // Update last login time
    auth_service.update_last_login(&user.id).await?;

    // Record login metrics
    state
        .metrics
        .record_api_key_login(&user.id, &user.subscription_tier.to_string());

    info!("Successful API key login for user: {}", user.id);

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        expires_in: token_expiry,
        token_type: "Bearer".to_string(),
        user: UserProfile {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user.name.clone(),
            avatar_url: user.avatar_url.clone(),
            subscription_tier: user.subscription_tier.clone(),
            roles: user.roles.clone(),
            permissions: user.permissions.clone(),
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login_at: Some(chrono::Utc::now()),
            email_verified: user.email_verified,
            two_factor_enabled: user.totp_secret.is_some(),
        },
    }))
}

/// POST /auth/refresh - Refresh access token
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>> {
    // Validate input
    payload.validate().map_err(|e| {
        ApiError::validation("refresh_token", format!("Invalid refresh request: {}", e))
    })?;

    info!("Token refresh attempt");

    // Validate refresh token and get user
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    let user = auth_service
        .validate_refresh_token(&payload.refresh_token)
        .await?
        .ok_or_else(|| ApiError::authentication("Invalid or expired refresh token"))?;

    // Check if account is still active
    if !user.is_active {
        return Err(ApiError::authentication("Account is disabled"));
    }

    // Generate new access token
    let token_expiry = 60 * 60; // 1 hour
    let access_token = auth_service
        .generate_access_token(&user, token_expiry)
        .await?;

    info!("Token refreshed successfully for user: {}", user.id);

    Ok(Json(RefreshTokenResponse {
        access_token,
        expires_in: token_expiry,
        token_type: "Bearer".to_string(),
    }))
}

/// POST /auth/logout - Logout and invalidate tokens
pub async fn logout(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
) -> Result<Json<LogoutResponse>> {
    info!("Logout request for user: {}", user_context.user_id);

    // TODO: Extract token from proper source (Extension or header extractor)
    // For now, we'll use a placeholder approach
    let token = "dummy_token"; // This should be extracted properly

    // Add token to blacklist
    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    // Blacklist the token to prevent reuse
    auth_service
        .blacklist_token(token, user_context.token_claims.exp)
        .await?;

    // Invalidate user refresh tokens
    auth_service
        .invalidate_user_refresh_tokens(&user_context.user_id)
        .await?;

    // Record logout metrics
    state.metrics.record_user_logout(&user_context.user_id);

    info!("Successful logout for user: {}", user_context.user_id);

    Ok(Json(LogoutResponse {
        message: "Logged out successfully".to_string(),
    }))
}

/// GET /auth/me - Get current user profile
pub async fn get_profile(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
) -> Result<Json<UserProfile>> {
    // Get fresh user data from database
    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    let user = auth_service
        .get_user_by_id(&user_context.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    Ok(Json(UserProfile {
        id: user.id.clone(),
        email: user.email.clone(),
        name: user.name.clone(),
        avatar_url: user.avatar_url.clone(),
        subscription_tier: user.subscription_tier.clone(),
        roles: user.roles.clone(),
        permissions: user.permissions.clone(),
        created_at: user.created_at,
        updated_at: user.updated_at,
        last_login_at: user.last_login_at,
        email_verified: user.email_verified,
        two_factor_enabled: user.totp_secret.is_some(),
    }))
}

/// PUT /auth/me - Update user profile
pub async fn update_profile(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>> {
    // Validate input
    payload
        .validate()
        .map_err(|e| ApiError::validation("profile", format!("Invalid profile update: {}", e)))?;

    info!("Profile update request for user: {}", user_context.user_id);

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    // Update user profile
    let updated_user = auth_service
        .update_user_profile(
            &user_context.user_id,
            payload.name.as_deref(),
            payload.avatar_url.as_deref(),
            payload.preferences.as_ref(),
        )
        .await?;

    info!(
        "Profile updated successfully for user: {}",
        user_context.user_id
    );

    Ok(Json(UserProfile {
        id: updated_user.id.clone(),
        email: updated_user.email.clone(),
        name: updated_user.name.clone(),
        avatar_url: updated_user.avatar_url.clone(),
        subscription_tier: updated_user.subscription_tier.clone(),
        roles: updated_user.roles.clone(),
        permissions: updated_user.permissions.clone(),
        created_at: updated_user.created_at,
        updated_at: updated_user.updated_at,
        last_login_at: updated_user.last_login_at,
        email_verified: updated_user.email_verified,
        two_factor_enabled: updated_user.totp_secret.is_some(),
    }))
}

/// PUT /auth/password - Change password
pub async fn change_password(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>> {
    // Validate input
    payload.validate().map_err(|e| {
        ApiError::validation(
            "password",
            format!("Invalid password change request: {}", e),
        )
    })?;

    info!("Password change request for user: {}", user_context.user_id);

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    // Verify current password
    let is_current_password_valid = auth_service
        .verify_password(&user_context.user_id, &payload.current_password)
        .await?;

    if !is_current_password_valid {
        return Err(ApiError::authentication("Current password is incorrect"));
    }

    // If 2FA is enabled, verify TOTP code
    // Verify TOTP if provided
    if let Some(totp_code) = &payload.totp_code {
        let is_totp_valid = auth_service
            .verify_totp(&user_context.user_id, totp_code)
            .await?;

        if !is_totp_valid {
            return Err(ApiError::authentication("Invalid 2FA code"));
        }
    }

    // Change password
    auth_service
        .change_password(&user_context.user_id, &payload.new_password)
        .await?;

    // Invalidate all sessions except current one
    let current_token = Some("current_token"); // This should be extracted from request
    auth_service
        .invalidate_user_sessions(&user_context.user_id, current_token)
        .await?;

    info!(
        "Password changed successfully for user: {}",
        user_context.user_id
    );

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}

/// GET /auth/sessions - Get active sessions for current user
pub async fn get_sessions(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
) -> Result<Json<Vec<UserSession>>> {
    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    let sessions = auth_service
        .get_user_sessions(&user_context.user_id)
        .await?;

    // Convert from service UserSession to handler UserSession
    let converted_sessions: Vec<UserSession> = sessions
        .into_iter()
        .map(|s| UserSession {
            session_id: s.session_id,
            device_info: s.device_info,
            ip_address: s.ip_address,
            user_agent: s.user_agent,
            created_at: s.created_at,
            last_accessed_at: s.last_accessed_at,
            is_current: s.is_current,
        })
        .collect();

    Ok(Json(converted_sessions))
}

/// DELETE /auth/sessions/{session_id} - Revoke a specific session
pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>> {
    info!(
        "Session revocation request for user: {}, session: {}",
        user_context.user_id, session_id
    );

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    auth_service
        .revoke_user_session(&user_context.user_id, &session_id)
        .await?;

    info!("Session revoked successfully: {}", session_id);

    Ok(Json(serde_json::json!({
        "message": "Session revoked successfully"
    })))
}

/// User session information
#[derive(Debug, Serialize)]
pub struct UserSession {
    pub session_id: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed_at: chrono::DateTime<chrono::Utc>,
    pub is_current: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_login_request_validation() {
        let valid_request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            totp_code: None,
            remember_me: false,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_email = LoginRequest {
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
            totp_code: None,
            remember_me: false,
        };
        assert!(invalid_email.validate().is_err());

        let short_password = LoginRequest {
            email: "test@example.com".to_string(),
            password: "short".to_string(),
            totp_code: None,
            remember_me: false,
        };
        assert!(short_password.validate().is_err());
    }

    #[test]
    fn test_api_key_request_validation() {
        let valid_request = ApiKeyLoginRequest {
            api_key: "a".repeat(32),
        };
        assert!(valid_request.validate().is_ok());

        let short_key = ApiKeyLoginRequest {
            api_key: "short".to_string(),
        };
        assert!(short_key.validate().is_err());
    }

    #[test]
    fn test_update_profile_validation() {
        let valid_request = UpdateProfileRequest {
            name: Some("John Doe".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            preferences: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_url = UpdateProfileRequest {
            name: Some("John Doe".to_string()),
            avatar_url: Some("not-a-url".to_string()),
            preferences: None,
        };
        assert!(invalid_url.validate().is_err());

        let empty_name = UpdateProfileRequest {
            name: Some("".to_string()),
            avatar_url: None,
            preferences: None,
        };
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_change_password_validation() {
        let valid_request = ChangePasswordRequest {
            current_password: "currentpass".to_string(),
            new_password: "newpassword123".to_string(),
            totp_code: None,
        };
        assert!(valid_request.validate().is_ok());

        let short_current = ChangePasswordRequest {
            current_password: "short".to_string(),
            new_password: "newpassword123".to_string(),
            totp_code: None,
        };
        assert!(short_current.validate().is_err());

        let short_new = ChangePasswordRequest {
            current_password: "currentpass".to_string(),
            new_password: "short".to_string(),
            totp_code: None,
        };
        assert!(short_new.validate().is_err());
    }
}

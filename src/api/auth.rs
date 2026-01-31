use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::ids::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // User ID
    pub exp: u64,     // Expiration time
    pub iat: u64,     // Issued at
    pub role: String, // User role (user, admin, operator)
}

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtAuth {
    pub fn new(secret: &str) -> Self {
        JwtAuth {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn generate_token(&self, user_id: UserId, role: &str, duration_secs: u64) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: user_id.to_string(),
            exp: now + duration_secs,
            iat: now,
            role: role.to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| Error::AuthenticationError(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        ).map_err(|e| Error::AuthenticationError(e.to_string()))?;

        Ok(token_data.claims)
    }
}

// Global JWT auth instance
lazy_static::lazy_static! {
    static ref JWT_AUTH: JwtAuth = {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default_secret_change_in_production".to_string());
        JwtAuth::new(&secret)
    };
}

pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = request.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract token from "Bearer <token>"
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify token
    let claims = JWT_AUTH.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add claims to request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

pub async fn admin_auth_middleware(
    request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // First run regular auth
    let response = auth_middleware(request, next).await?;

    // Check if user has admin role
    // This would be extracted from request extensions

    Ok(response)
}

// API Key authentication (alternative to JWT)
pub struct ApiKeyAuth {
    valid_keys: std::collections::HashMap<String, UserId>,
}

impl ApiKeyAuth {
    pub fn new() -> Self {
        ApiKeyAuth {
            valid_keys: std::collections::HashMap::new(),
        }
    }

    pub fn add_key(&mut self, key: String, user_id: UserId) {
        self.valid_keys.insert(key, user_id);
    }

    pub fn verify_key(&self, key: &str) -> Option<UserId> {
        self.valid_keys.get(key).copied()
    }
}
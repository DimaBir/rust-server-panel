use actix_web::{dev::ServiceRequest, web, Error, HttpMessage, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};

use crate::config::AppConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // username
    pub exp: usize,  // expiry timestamp
    pub iat: usize,  // issued at
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub username: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

/// Create a JWT token for the given username.
fn create_token(username: &str, secret: &str) -> anyhow::Result<(String, chrono::DateTime<Utc>)> {
    let expires_at = Utc::now() + Duration::hours(24);
    let claims = Claims {
        sub: username.to_string(),
        exp: expires_at.timestamp() as usize,
        iat: Utc::now().timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok((token, expires_at))
}

/// Validate a JWT token and return the claims.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// POST /api/auth/login
pub async fn login(
    body: web::Json<LoginRequest>,
    config: web::Data<AppConfig>,
) -> HttpResponse {
    // Verify username
    if body.username != config.auth.admin_username {
        return HttpResponse::Unauthorized().json(ErrorBody {
            error: "Invalid credentials".to_string(),
        });
    }

    // Verify password against bcrypt hash
    match bcrypt::verify(&body.password, &config.auth.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Unauthorized().json(ErrorBody {
                error: "Invalid credentials".to_string(),
            });
        }
        Err(e) => {
            tracing::error!("Bcrypt verification error: {}", e);
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "Authentication error".to_string(),
            });
        }
    }

    // Create JWT
    match create_token(&body.username, &config.auth.jwt_secret) {
        Ok((token, expires_at)) => HttpResponse::Ok().json(LoginResponse {
            token,
            username: body.username.clone(),
            expires_at: expires_at.to_rfc3339(),
        }),
        Err(e) => {
            tracing::error!("Token creation error: {}", e);
            HttpResponse::InternalServerError().json(ErrorBody {
                error: "Token creation failed".to_string(),
            })
        }
    }
}

/// GET /api/auth/me
pub async fn me(req: HttpRequest) -> HttpResponse {
    if let Some(claims) = req.extensions().get::<Claims>() {
        HttpResponse::Ok().json(MeResponse {
            username: claims.sub.clone(),
            role: "admin".to_string(),
        })
    } else {
        HttpResponse::Unauthorized().json(ErrorBody {
            error: "Not authenticated".to_string(),
        })
    }
}

/// Extract Bearer token from Authorization header.
fn extract_bearer_token(req: &ServiceRequest) -> Option<String> {
    let auth_header = req.headers().get("Authorization")?.to_str().ok()?;
    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

/// Actix-web middleware for JWT authentication.
/// Protects all routes except /api/auth/login.
pub struct JwtAuth;

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for JwtAuth
where
    S: actix_web::dev::Service<ServiceRequest, Response = actix_web::dev::ServiceResponse<B>, Error = Error>
        + 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = actix_web::dev::ServiceResponse<B>, Error = Error>
        + 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &self,
        ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let path = req.path().to_string();

            // Skip auth for login endpoint, WebSocket upgrades, position updates (uses RCON token), and static files
            let is_public = path == "/api/auth/login"
                || path.starts_with("/ws/")
                || !path.starts_with("/api/")
                || (req.method() == actix_web::http::Method::POST && path.ends_with("/positions"));

            if is_public {
                return service.call(req).await;
            }

            // Extract and validate token
            let token = match extract_bearer_token(&req) {
                Some(t) => t,
                None => {
                    return Err(actix_web::error::ErrorUnauthorized(
                        r#"{"error":"Missing authorization token"}"#,
                    ));
                }
            };

            // Get JWT secret from app data
            let config = match req.app_data::<web::Data<AppConfig>>() {
                Some(c) => c.clone(),
                None => {
                    return Err(actix_web::error::ErrorInternalServerError(
                        r#"{"error":"Server configuration error"}"#,
                    ));
                }
            };

            match validate_token(&token, &config.auth.jwt_secret) {
                Ok(claims) => {
                    req.extensions_mut().insert(claims);
                    service.call(req).await
                }
                Err(e) => {
                    tracing::debug!("JWT validation failed: {}", e);
                    Err(actix_web::error::ErrorUnauthorized(
                        r#"{"error":"Invalid or expired token"}"#,
                    ))
                }
            }
        })
    }
}

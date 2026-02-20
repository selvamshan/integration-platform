//! Keycloak integration for authentication and user management

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use common::{User, UserRole};

/// Keycloak configuration
#[derive(Clone)]
pub struct KeycloakConfig {
    pub server_url: String,      // e.g., http://keycloak:8080
    pub realm: String,            // e.g., integration-platform
    pub client_id: String,
    pub client_secret: String,
    pub http_client: Client,
}

/// Keycloak JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct KeycloakClaims {
    pub sub: String,                    // User ID
    pub preferred_username: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub realm_access: Option<RealmAccess>,
    pub resource_access: Option<serde_json::Value>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RealmAccess {
    pub roles: Vec<String>,
}

impl KeycloakConfig {
    pub fn from_env() -> Result<Self> {
        let server_url = std::env::var("KEYCLOAK_SERVER_URL")
            .unwrap_or_else(|_| "http://keycloak:8080".to_string());
        let realm = std::env::var("KEYCLOAK_REALM")
            .unwrap_or_else(|_| "integration-platform".to_string());
        let client_id = std::env::var("KEYCLOAK_CLIENT_ID")
            .unwrap_or_else(|_| "control-plane".to_string());
        let client_secret = std::env::var("KEYCLOAK_CLIENT_SECRET")
            .map_err(|_| anyhow!("KEYCLOAK_CLIENT_SECRET env var required"))?;

        Ok(Self {
            server_url,
            realm,
            client_id,
            client_secret,
            http_client: Client::new(),
        })
    }

    /// Get Keycloak public key for JWT verification
    pub async fn get_public_key(&self) -> Result<DecodingKey> {
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/certs",
            self.server_url, self.realm
        );

        let resp: serde_json::Value = self.http_client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        // Get first RSA key
        let keys = resp["keys"].as_array()
            .ok_or_else(|| anyhow!("No keys in JWKS"))?;

        let first_key = keys.first()
            .ok_or_else(|| anyhow!("Empty JWKS"))?;

        let n = first_key["n"].as_str()
            .ok_or_else(|| anyhow!("Missing 'n' in key"))?;
        let e = first_key["e"].as_str()
            .ok_or_else(|| anyhow!("Missing 'e' in key"))?;

        DecodingKey::from_rsa_components(n, e)
            .map_err(|e| anyhow!("Failed to create decoding key: {}", e))
    }

    /// Validate Keycloak JWT and extract user info
    pub async fn validate_token(&self, token: &str) -> Result<User> {
        let public_key = self.get_public_key().await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&["account", &self.client_id]);
        validation.validate_exp = true;

        let token_data = decode::<KeycloakClaims>(token, &public_key, &validation)
            .map_err(|e| anyhow!("Token validation failed: {}", e))?;

        let claims = token_data.claims;

        // Extract roles from both realm_access and resource_access (client roles)
        let mut roles = Vec::new();
        
        // Check realm-level roles
        if let Some(realm_access) = claims.realm_access {
            for role in realm_access.roles {
                if let Some(user_role) = UserRole::from_str(&role) {
                    roles.push(user_role);
                }
            }
        }
        
        // Check client-level roles (resource_access.control-plane.roles)
        if let Some(resource_access) = claims.resource_access {
            if let Some(client_roles) = resource_access.get(&self.client_id) {
                if let Some(role_array) = client_roles.get("roles").and_then(|v| v.as_array()) {
                    for role_value in role_array {
                        if let Some(role_str) = role_value.as_str() {
                            if let Some(user_role) = UserRole::from_str(role_str) {
                                roles.push(user_role);
                            }
                        }
                    }
                }
            }
        }
        
        // Default to viewer if no roles found
        if roles.is_empty() {
            roles.push(UserRole::Viewer);
        }

        Ok(User {
            id: claims.sub,
            username: claims.preferred_username,
            email: claims.email.unwrap_or_default(),
            name: claims.name,
            roles,
            created_at: chrono::Utc::now(), // Keycloak doesn't provide this in JWT
        })
    }

    /// Invite user via Keycloak Admin API
    pub async fn invite_user(&self, email: &str, role: &UserRole) -> Result<String> {
        // Get admin access token
        let admin_token = self.get_admin_token().await?;

        // Create user in Keycloak
        let create_user_url = format!(
            "{}/admin/realms/{}/users",
            self.server_url, self.realm
        );

        let username = email.split('@').next().unwrap_or(email);

        let user_payload = serde_json::json!({
            "username": username,
            "email": email,
            "enabled": true,
            "emailVerified": false,
            "requiredActions": ["VERIFY_EMAIL", "UPDATE_PASSWORD"]
        });

        let create_resp = self.http_client
            .post(&create_user_url)
            .bearer_auth(&admin_token)
            .json(&user_payload)
            .send()
            .await?;

        if !create_resp.status().is_success() {
            let error_text = create_resp.text().await?;
            return Err(anyhow!("Failed to create user: {}", error_text));
        }

        // Get user ID from Location header
        let location = create_resp.headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| anyhow!("No Location header in response"))?;

        let user_id = location.split('/').last()
            .ok_or_else(|| anyhow!("Invalid Location header"))?;

        // Assign role
        self.assign_role(user_id, role, &admin_token).await?;

        // Send verification email
        self.send_verify_email(user_id, &admin_token).await?;

        Ok(user_id.to_string())
    }

    /// Get admin access token using client credentials
    async fn get_admin_token(&self) -> Result<String> {
        let token_url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.server_url, self.realm
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let resp: serde_json::Value = self.http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        resp["access_token"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No access_token in response"))
    }

    /// Assign role to user
    async fn assign_role(&self, user_id: &str, role: &UserRole, admin_token: &str) -> Result<()> {
        // Get role representation
        let role_url = format!(
            "{}/admin/realms/{}/roles/{}",
            self.server_url, self.realm, role.as_str()
        );

        let role_resp: serde_json::Value = self.http_client
            .get(&role_url)
            .bearer_auth(admin_token)
            .send()
            .await?
            .json()
            .await?;

        // Assign role to user
        let assign_url = format!(
            "{}/admin/realms/{}/users/{}/role-mappings/realm",
            self.server_url, self.realm, user_id
        );

        let assign_payload = serde_json::json!([role_resp]);

        let assign_resp = self.http_client
            .post(&assign_url)
            .bearer_auth(admin_token)
            .json(&assign_payload)
            .send()
            .await?;

        if !assign_resp.status().is_success() {
            return Err(anyhow!("Failed to assign role"));
        }

        Ok(())
    }

    /// Send email verification
    async fn send_verify_email(&self, user_id: &str, admin_token: &str) -> Result<()> {
        let verify_url = format!(
            "{}/admin/realms/{}/users/{}/send-verify-email",
            self.server_url, self.realm, user_id
        );

        let resp = self.http_client
            .put(&verify_url)
            .bearer_auth(admin_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Failed to send verification email for user {}", user_id);
        }

        Ok(())
    }

    /// List users (admin only)
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let admin_token = self.get_admin_token().await?;

        let list_url = format!(
            "{}/admin/realms/{}/users",
            self.server_url, self.realm
        );

        let users_resp: Vec<serde_json::Value> = self.http_client
            .get(&list_url)
            .bearer_auth(&admin_token)
            .send()
            .await?
            .json()
            .await?;

        let mut users = Vec::new();

        for user_data in users_resp {
            let user_id = user_data["id"].as_str().unwrap_or_default().to_string();
            
            // Get user roles
            let roles_url = format!(
                "{}/admin/realms/{}/users/{}/role-mappings/realm",
                self.server_url, self.realm, user_id
            );

            let roles_resp: Vec<serde_json::Value> = self.http_client
                .get(&roles_url)
                .bearer_auth(&admin_token)
                .send()
                .await?
                .json()
                .await
                .unwrap_or_default();

            let roles = roles_resp.iter()
                .filter_map(|r| r["name"].as_str())
                .filter_map(UserRole::from_str)
                .collect();

            users.push(User {
                id: user_id,
                username: user_data["username"].as_str().unwrap_or_default().to_string(),
                email: user_data["email"].as_str().unwrap_or_default().to_string(),
                name: user_data["firstName"].as_str().map(|s| s.to_string()),
                roles,
                created_at: chrono::Utc::now(),
            });
        }

        Ok(users)
    }

    /// Delete user (admin only)
    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        let admin_token = self.get_admin_token().await?;

        let delete_url = format!(
            "{}/admin/realms/{}/users/{}",
            self.server_url, self.realm, user_id
        );

        let resp = self.http_client
            .delete(&delete_url)
            .bearer_auth(&admin_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Failed to delete user"));
        }

        Ok(())
    }
}

//! Unified OIDC authentication supporting Keycloak, Auth0, and Okta.
//!
//! Provider is selected via the `OIDC_PROVIDER` env var (keycloak|auth0|okta).
//! Each provider has its own JWKS endpoint, claims structure, and management API.

use anyhow::{Result, anyhow};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use common::{User, UserRole};

// ── Provider configuration ────────────────────────────────────────────────────

#[derive(Clone)]
pub enum OidcProviderConfig {
    Keycloak {
        server_url: String,
        realm: String,
        client_id: String,
        client_secret: String,
    },
    Auth0 {
        domain: String,
        client_id: String,
        client_secret: String,
        audience: String,
        roles_namespace: String,
    },
    Okta {
        domain: String,
        auth_server_id: String,
        client_id: String,
        client_secret: String,
        audience: String,
    },
}

// ── Generic OIDC claims (covers all three providers) ─────────────────────────

#[derive(Debug, Deserialize)]
struct OidcClaims {
    sub: String,
    preferred_username: Option<String>,
    email: Option<String>,
    name: Option<String>,
    // Keycloak: realm-level roles
    realm_access: Option<RealmAccess>,
    // Keycloak: client-level roles
    resource_access: Option<serde_json::Value>,
    // Auth0 / Okta: top-level roles array
    roles: Option<Vec<String>>,
    // Okta: groups claim
    groups: Option<Vec<String>>,
    // Catch-all for Auth0 namespace claims and other extras
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RealmAccess {
    roles: Vec<String>,
}

// ── Unified auth service ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct OidcAuth {
    pub config: OidcProviderConfig,
    pub http: Client,
}

impl OidcAuth {
    /// Load provider config from environment variables.
    /// Falls back to a Keycloak-shaped empty config if nothing is configured,
    /// so the binary still starts without crashing.
    pub fn from_env() -> Self {
        match Self::try_from_env() {
            Ok(auth) => auth,
            Err(e) => {
                tracing::warn!("⚠️  OIDC not fully configured: {e} — auth will fail at runtime");
                Self {
                    config: OidcProviderConfig::Keycloak {
                        server_url: String::new(),
                        realm: String::new(),
                        client_id: String::new(),
                        client_secret: String::new(),
                    },
                    http: Client::new(),
                }
            }
        }
    }

    fn try_from_env() -> Result<Self> {
        let provider = std::env::var("OIDC_PROVIDER")
            .unwrap_or_else(|_| "keycloak".to_string());

        let config = match provider.to_lowercase().as_str() {
            "auth0" => OidcProviderConfig::Auth0 {
                domain: std::env::var("AUTH0_DOMAIN")
                    .map_err(|_| anyhow!("AUTH0_DOMAIN required"))?,
                client_id: std::env::var("AUTH0_CLIENT_ID")
                    .map_err(|_| anyhow!("AUTH0_CLIENT_ID required"))?,
                client_secret: std::env::var("AUTH0_CLIENT_SECRET")
                    .map_err(|_| anyhow!("AUTH0_CLIENT_SECRET required"))?,
                audience: std::env::var("AUTH0_AUDIENCE")
                    .map_err(|_| anyhow!("AUTH0_AUDIENCE required"))?,
                roles_namespace: std::env::var("AUTH0_ROLES_NAMESPACE")
                    .unwrap_or_else(|_| "https://integration-platform/roles".to_string()),
            },
            "okta" => OidcProviderConfig::Okta {
                domain: std::env::var("OKTA_DOMAIN")
                    .map_err(|_| anyhow!("OKTA_DOMAIN required"))?,
                auth_server_id: std::env::var("OKTA_AUTH_SERVER_ID")
                    .unwrap_or_else(|_| "default".to_string()),
                client_id: std::env::var("OKTA_CLIENT_ID")
                    .map_err(|_| anyhow!("OKTA_CLIENT_ID required"))?,
                client_secret: std::env::var("OKTA_CLIENT_SECRET")
                    .map_err(|_| anyhow!("OKTA_CLIENT_SECRET required"))?,
                audience: std::env::var("OKTA_AUDIENCE")
                    .unwrap_or_else(|_| "api://default".to_string()),
            },
            _ => OidcProviderConfig::Keycloak {
                server_url: std::env::var("KEYCLOAK_SERVER_URL")
                    .unwrap_or_else(|_| "http://keycloak:8080".to_string()),
                realm: std::env::var("KEYCLOAK_REALM")
                    .unwrap_or_else(|_| "integration-platform".to_string()),
                client_id: std::env::var("KEYCLOAK_CLIENT_ID")
                    .unwrap_or_else(|_| "control-plane".to_string()),
                client_secret: std::env::var("KEYCLOAK_CLIENT_SECRET")
                    .map_err(|_| anyhow!("KEYCLOAK_CLIENT_SECRET required"))?,
            },
        };

        tracing::info!("✅ OIDC provider: {}", provider);
        Ok(Self { config, http: Client::new() })
    }

    pub fn provider_name(&self) -> &'static str {
        match &self.config {
            OidcProviderConfig::Keycloak { .. } => "keycloak",
            OidcProviderConfig::Auth0 { .. } => "auth0",
            OidcProviderConfig::Okta { .. } => "okta",
        }
    }

    fn jwks_url(&self) -> String {
        match &self.config {
            OidcProviderConfig::Keycloak { server_url, realm, .. } =>
                format!("{}/realms/{}/protocol/openid-connect/certs", server_url, realm),
            OidcProviderConfig::Auth0 { domain, .. } =>
                format!("https://{}/.well-known/jwks.json", domain),
            OidcProviderConfig::Okta { domain, auth_server_id, .. } =>
                format!("https://{}/oauth2/{}/v1/keys", domain, auth_server_id),
        }
    }

    async fn fetch_jwks(&self) -> Result<serde_json::Value> {
        self.http
            .get(self.jwks_url())
            .send().await?
            .json().await
            .map_err(|e| anyhow!("Failed to fetch JWKS: {}", e))
    }

    fn find_key<'a>(keys: &'a [serde_json::Value], kid: Option<&str>) -> Option<&'a serde_json::Value> {
        if let Some(kid) = kid {
            // Match by key ID — critical when Keycloak has multiple keys (rotation)
            if let Some(k) = keys.iter().find(|k| k["kid"].as_str() == Some(kid)) {
                return Some(k);
            }
        }
        // Fallback: first RSA key
        keys.iter().find(|k| k["kty"].as_str() == Some("RSA"))
    }

    fn key_from_jwks_entry(rsa_key: &serde_json::Value) -> Result<DecodingKey> {
        let n = rsa_key["n"].as_str().ok_or_else(|| anyhow!("Missing 'n' in JWKS key"))?;
        let e = rsa_key["e"].as_str().ok_or_else(|| anyhow!("Missing 'e' in JWKS key"))?;
        DecodingKey::from_rsa_components(n, e)
            .map_err(|e| anyhow!("Failed to build decoding key: {}", e))
    }

    // ── Token validation ──────────────────────────────────────────────────────

    pub async fn validate_token(&self, token: &str) -> Result<User> {
        // Decode the JWT header (unverified) to get `kid`
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| anyhow!("Invalid JWT header: {}", e))?;

        let jwks = self.fetch_jwks().await?;
        let keys = jwks["keys"].as_array()
            .ok_or_else(|| anyhow!("No keys in JWKS ({})", self.provider_name()))?;

        let rsa_key = Self::find_key(keys, header.kid.as_deref())
            .ok_or_else(|| anyhow!("No matching key in JWKS for kid={:?}", header.kid))?;

        let key = Self::key_from_jwks_entry(rsa_key)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        match &self.config {
            // Keycloak access tokens often omit `aud` unless audience mappers are
            // explicitly configured in the realm. Signature + expiry is sufficient.
            OidcProviderConfig::Keycloak { .. } => {
                validation.validate_aud = false;
            }
            OidcProviderConfig::Auth0 { audience, .. } =>
                validation.set_audience(&[audience.as_str()]),
            OidcProviderConfig::Okta { audience, .. } =>
                validation.set_audience(&[audience.as_str()]),
        }

        let claims = decode::<OidcClaims>(token, &key, &validation)
            .map_err(|e| anyhow!("Token invalid ({}): {}", self.provider_name(), e))?
            .claims;

        let roles = self.extract_roles(&claims);

        let username = claims.preferred_username
            .or_else(|| claims.email.clone())
            .unwrap_or_else(|| claims.sub.clone());

        Ok(User {
            id: claims.sub,
            username,
            email: claims.email.unwrap_or_default(),
            name: claims.name,
            roles,
            created_at: chrono::Utc::now(),
        })
    }

    fn extract_roles(&self, claims: &OidcClaims) -> Vec<UserRole> {
        let mut roles = Vec::new();

        match &self.config {
            OidcProviderConfig::Keycloak { client_id, .. } => {
                if let Some(ra) = &claims.realm_access {
                    roles.extend(ra.roles.iter().filter_map(|r| UserRole::from_str(r)));
                }
                if let Some(ra) = &claims.resource_access {
                    if let Some(arr) = ra[client_id]["roles"].as_array() {
                        roles.extend(arr.iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(UserRole::from_str));
                    }
                }
            }
            OidcProviderConfig::Auth0 { roles_namespace, .. } => {
                // Custom namespace claim (e.g. "https://integration-platform/roles")
                if let Some(ns) = claims.extra.get(roles_namespace) {
                    if let Some(arr) = ns.as_array() {
                        roles.extend(arr.iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(UserRole::from_str));
                    }
                }
                // Top-level roles (if Actions rule copies them there)
                if let Some(r) = &claims.roles {
                    roles.extend(r.iter().filter_map(|s| UserRole::from_str(s)));
                }
            }
            OidcProviderConfig::Okta { .. } => {
                // Groups claim (configured in Okta Authorization Server)
                if let Some(g) = &claims.groups {
                    roles.extend(g.iter().filter_map(|s| UserRole::from_str(s)));
                }
                if let Some(r) = &claims.roles {
                    roles.extend(r.iter().filter_map(|s| UserRole::from_str(s)));
                }
            }
        }

        if roles.is_empty() {
            roles.push(UserRole::Viewer);
        }
        roles.dedup();
        roles
    }

    // ── User management (dispatches to provider-specific impl) ───────────────

    pub async fn invite_user(&self, email: &str, role: &UserRole) -> Result<String> {
        match &self.config {
            OidcProviderConfig::Keycloak { .. } => self.kc_invite(email, role).await,
            OidcProviderConfig::Auth0 { .. }    => self.auth0_invite(email, role).await,
            OidcProviderConfig::Okta { .. }     => self.okta_invite(email, role).await,
        }
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        match &self.config {
            OidcProviderConfig::Keycloak { .. } => self.kc_list_users().await,
            OidcProviderConfig::Auth0 { .. }    => self.auth0_list_users().await,
            OidcProviderConfig::Okta { .. }     => self.okta_list_users().await,
        }
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        match &self.config {
            OidcProviderConfig::Keycloak { .. } => self.kc_delete_user(user_id).await,
            OidcProviderConfig::Auth0 { .. }    => self.auth0_delete_user(user_id).await,
            OidcProviderConfig::Okta { .. }     => self.okta_delete_user(user_id).await,
        }
    }

    // ── Keycloak management ───────────────────────────────────────────────────

    async fn kc_admin_token(&self) -> Result<String> {
        let OidcProviderConfig::Keycloak { server_url, realm, client_id, client_secret } = &self.config
            else { return Err(anyhow!("Not Keycloak")); };

        let url = format!("{}/realms/{}/protocol/openid-connect/token", server_url, realm);
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ];
        let resp: serde_json::Value = self.http.post(&url).form(&params).send().await?.json().await?;
        resp["access_token"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No access_token from Keycloak admin"))
    }

    async fn kc_invite(&self, email: &str, role: &UserRole) -> Result<String> {
        let OidcProviderConfig::Keycloak { server_url, realm, .. } = &self.config
            else { return Err(anyhow!("Not Keycloak")); };

        let tok = self.kc_admin_token().await?;
        let username = email.split('@').next().unwrap_or(email);
        let create_url = format!("{}/admin/realms/{}/users", server_url, realm);
        let payload = serde_json::json!({
            "username": username, "email": email, "enabled": true,
            "emailVerified": false,
            "requiredActions": ["VERIFY_EMAIL", "UPDATE_PASSWORD"],
        });
        let resp = self.http.post(&create_url).bearer_auth(&tok).json(&payload).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Keycloak create user: {}", resp.text().await?));
        }
        let location = resp.headers().get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| anyhow!("No Location header"))?
            .to_string();
        let user_id = location.split('/').last().unwrap_or("").to_string();

        // Assign role
        let role_url = format!("{}/admin/realms/{}/roles/{}", server_url, realm, role.as_str());
        let role_rep: serde_json::Value = self.http.get(&role_url).bearer_auth(&tok).send().await?.json().await?;
        let assign_url = format!("{}/admin/realms/{}/users/{}/role-mappings/realm", server_url, realm, user_id);
        self.http.post(&assign_url).bearer_auth(&tok).json(&serde_json::json!([role_rep])).send().await?;

        // Verification email (best-effort)
        let verify_url = format!("{}/admin/realms/{}/users/{}/send-verify-email", server_url, realm, user_id);
        let _ = self.http.put(&verify_url).bearer_auth(&tok).send().await;

        Ok(user_id)
    }

    async fn kc_list_users(&self) -> Result<Vec<User>> {
        let OidcProviderConfig::Keycloak { server_url, realm, .. } = &self.config
            else { return Err(anyhow!("Not Keycloak")); };

        let tok = self.kc_admin_token().await?;
        let url = format!("{}/admin/realms/{}/users", server_url, realm);
        let raw: Vec<serde_json::Value> = self.http.get(&url).bearer_auth(&tok).send().await?.json().await?;
        let mut users = Vec::new();
        for u in raw {
            let uid = u["id"].as_str().unwrap_or_default().to_string();
            let roles_url = format!("{}/admin/realms/{}/users/{}/role-mappings/realm", server_url, realm, uid);
            let roles_raw: Vec<serde_json::Value> = self.http.get(&roles_url).bearer_auth(&tok).send().await?.json().await.unwrap_or_default();
            let roles = roles_raw.iter().filter_map(|r| r["name"].as_str()).filter_map(UserRole::from_str).collect();
            users.push(User {
                id: uid,
                username: u["username"].as_str().unwrap_or_default().to_string(),
                email: u["email"].as_str().unwrap_or_default().to_string(),
                name: u["firstName"].as_str().map(|s| s.to_string()),
                roles,
                created_at: chrono::Utc::now(),
            });
        }
        Ok(users)
    }

    async fn kc_delete_user(&self, user_id: &str) -> Result<()> {
        let OidcProviderConfig::Keycloak { server_url, realm, .. } = &self.config
            else { return Err(anyhow!("Not Keycloak")); };

        let tok = self.kc_admin_token().await?;
        let url = format!("{}/admin/realms/{}/users/{}", server_url, realm, user_id);
        let resp = self.http.delete(&url).bearer_auth(&tok).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Keycloak delete user failed"));
        }
        Ok(())
    }

    // ── Auth0 management ──────────────────────────────────────────────────────

    async fn auth0_mgmt_token(&self) -> Result<String> {
        let OidcProviderConfig::Auth0 { domain, client_id, client_secret, .. } = &self.config
            else { return Err(anyhow!("Not Auth0")); };

        let url = format!("https://{}/oauth/token", domain);
        let payload = serde_json::json!({
            "client_id":     client_id,
            "client_secret": client_secret,
            "audience":      format!("https://{}/api/v2/", domain),
            "grant_type":    "client_credentials",
        });
        let resp: serde_json::Value = self.http.post(&url).json(&payload).send().await?.json().await?;
        resp["access_token"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No access_token from Auth0 Management API"))
    }

    async fn auth0_invite(&self, email: &str, role: &UserRole) -> Result<String> {
        let OidcProviderConfig::Auth0 { domain, .. } = &self.config
            else { return Err(anyhow!("Not Auth0")); };

        let tok = self.auth0_mgmt_token().await?;
        let create_url = format!("https://{}/api/v2/users", domain);
        let temp_pwd = format!("Temp@{}!", uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string());
        let payload = serde_json::json!({
            "email": email,
            "connection": "Username-Password-Authentication",
            "password": temp_pwd,
            "email_verified": false,
            "verify_email": true,
        });
        let resp = self.http.post(&create_url).bearer_auth(&tok).json(&payload).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Auth0 create user: {}", resp.text().await?));
        }
        let user_data: serde_json::Value = resp.json().await?;
        let user_id = user_data["user_id"].as_str()
            .ok_or_else(|| anyhow!("No user_id from Auth0"))?.to_string();

        // Find role by name and assign
        let roles_url = format!("https://{}/api/v2/roles?name_filter={}", domain, role.as_str());
        let roles_data: Vec<serde_json::Value> = self.http.get(&roles_url).bearer_auth(&tok).send().await?.json().await.unwrap_or_default();
        if let Some(role_id) = roles_data.first().and_then(|r| r["id"].as_str()) {
            let assign_url = format!("https://{}/api/v2/users/{}/roles", domain, user_id);
            self.http.post(&assign_url).bearer_auth(&tok)
                .json(&serde_json::json!({"roles": [role_id]}))
                .send().await?;
        } else {
            tracing::warn!("Auth0 role '{}' not found — user created without role", role.as_str());
        }

        Ok(user_id)
    }

    async fn auth0_list_users(&self) -> Result<Vec<User>> {
        let OidcProviderConfig::Auth0 { domain, .. } = &self.config
            else { return Err(anyhow!("Not Auth0")); };

        let tok = self.auth0_mgmt_token().await?;
        let url = format!("https://{}/api/v2/users?per_page=100&include_totals=false", domain);
        let raw: Vec<serde_json::Value> = self.http.get(&url).bearer_auth(&tok).send().await?.json().await?;
        Ok(raw.into_iter().map(|u| User {
            id: u["user_id"].as_str().unwrap_or_default().to_string(),
            username: u["email"].as_str().unwrap_or_default().to_string(),
            email: u["email"].as_str().unwrap_or_default().to_string(),
            name: u["name"].as_str().map(|s| s.to_string()),
            roles: vec![UserRole::Viewer],
            created_at: chrono::Utc::now(),
        }).collect())
    }

    async fn auth0_delete_user(&self, user_id: &str) -> Result<()> {
        let OidcProviderConfig::Auth0 { domain, .. } = &self.config
            else { return Err(anyhow!("Not Auth0")); };

        let tok = self.auth0_mgmt_token().await?;
        let url = format!("https://{}/api/v2/users/{}", domain, user_id);
        let resp = self.http.delete(&url).bearer_auth(&tok).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Auth0 delete user failed"));
        }
        Ok(())
    }

    // ── Okta management ───────────────────────────────────────────────────────

    fn okta_ssws_token(&self) -> Result<String> {
        // Okta management uses an SSWS API token set separately
        std::env::var("OKTA_API_TOKEN")
            .map_err(|_| anyhow!("OKTA_API_TOKEN env var required for Okta management"))
    }

    async fn okta_invite(&self, email: &str, role: &UserRole) -> Result<String> {
        let OidcProviderConfig::Okta { domain, .. } = &self.config
            else { return Err(anyhow!("Not Okta")); };

        let ssws = self.okta_ssws_token()?;
        let username = email.split('@').next().unwrap_or(email);
        let create_url = format!("https://{}/api/v1/users?activate=true", domain);
        let payload = serde_json::json!({
            "profile": { "firstName": username, "lastName": "", "email": email, "login": email },
        });
        let resp = self.http.post(&create_url)
            .header("Authorization", format!("SSWS {}", ssws))
            .header("Accept", "application/json")
            .json(&payload)
            .send().await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Okta create user: {}", resp.text().await?));
        }
        let user_data: serde_json::Value = resp.json().await?;
        let user_id = user_data["id"].as_str()
            .ok_or_else(|| anyhow!("No id from Okta"))?.to_string();

        // Find group matching role name and add user
        let groups_url = format!("https://{}/api/v1/groups?q={}", domain, role.as_str());
        let groups: Vec<serde_json::Value> = self.http.get(&groups_url)
            .header("Authorization", format!("SSWS {}", ssws))
            .header("Accept", "application/json")
            .send().await?.json().await.unwrap_or_default();

        if let Some(group_id) = groups.first().and_then(|g| g["id"].as_str()) {
            let add_url = format!("https://{}/api/v1/groups/{}/users/{}", domain, group_id, user_id);
            let _ = self.http.put(&add_url)
                .header("Authorization", format!("SSWS {}", ssws))
                .send().await;
        } else {
            tracing::warn!("Okta group '{}' not found — user created without group", role.as_str());
        }

        Ok(user_id)
    }

    async fn okta_list_users(&self) -> Result<Vec<User>> {
        let OidcProviderConfig::Okta { domain, .. } = &self.config
            else { return Err(anyhow!("Not Okta")); };

        let ssws = self.okta_ssws_token()?;
        let url = format!("https://{}/api/v1/users?limit=200", domain);
        let raw: Vec<serde_json::Value> = self.http.get(&url)
            .header("Authorization", format!("SSWS {}", ssws))
            .header("Accept", "application/json")
            .send().await?.json().await?;

        Ok(raw.into_iter().map(|u| {
            let p = &u["profile"];
            User {
                id: u["id"].as_str().unwrap_or_default().to_string(),
                username: p["login"].as_str().unwrap_or_default().to_string(),
                email: p["email"].as_str().unwrap_or_default().to_string(),
                name: p["firstName"].as_str().map(|s| s.to_string()),
                roles: vec![UserRole::Viewer],
                created_at: chrono::Utc::now(),
            }
        }).collect())
    }

    async fn okta_delete_user(&self, user_id: &str) -> Result<()> {
        let OidcProviderConfig::Okta { domain, .. } = &self.config
            else { return Err(anyhow!("Not Okta")); };

        let ssws = self.okta_ssws_token()?;
        // Okta requires deactivation before deletion
        let deactivate = format!("https://{}/api/v1/users/{}/lifecycle/deactivate", domain, user_id);
        let _ = self.http.post(&deactivate)
            .header("Authorization", format!("SSWS {}", ssws))
            .send().await;

        let delete_url = format!("https://{}/api/v1/users/{}", domain, user_id);
        let resp = self.http.delete(&delete_url)
            .header("Authorization", format!("SSWS {}", ssws))
            .send().await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Okta delete user failed"));
        }
        Ok(())
    }
}

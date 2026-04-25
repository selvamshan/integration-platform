use anyhow::Result;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::state::AppState;

pub async fn credential_validation_service(state: Arc<AppState>) -> Result<()> {
    let mut subscriber = state.nats.subscribe("auth.validate.credentials").await?;
    tracing::info!("🔐 Credential validation service started");

    while let Some(msg) = subscriber.next().await {
        let reply = match msg.reply { Some(r) => r, None => continue };

        let response = match do_validate_credentials(&state, &msg.payload).await {
            Ok(p) => {
                tracing::debug!("✅ Credential OK: {}", p.client_id);
                json!({ "valid": true, "client_id": p.client_id, "name": p.client_name })
            }
            Err(reason) => {
                tracing::warn!("❌ Credential rejected: {}", reason);
                json!({ "valid": false, "reason": reason })
            }
        };

        let _ = state.nats.publish(reply, serde_json::to_vec(&response).unwrap_or_default().into()).await;
    }

    Ok(())
}

async fn do_validate_credentials(
    state: &AppState,
    payload: &[u8],
) -> std::result::Result<common::AuthPrincipal, String> {
    #[derive(Deserialize)]
    struct Req { client_id: String, client_secret: String }

    let req: Req = serde_json::from_slice(payload).map_err(|_| "Malformed request".to_string())?;

    let row = sqlx::query!(
        "SELECT client_id, client_secret_hash, name, active, expires_at
         FROM client_credentials WHERE client_id = $1", req.client_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Client not found".to_string())?;

    if !row.active { return Err("Client is deactivated".into()); }
    if let Some(exp) = row.expires_at {
        if exp < chrono::Utc::now() { return Err("Credential expired".into()); }
    }
    if !bcrypt::verify(&req.client_secret, &row.client_secret_hash).map_err(|e| e.to_string())? {
        return Err("Invalid secret".into());
    }

    Ok(common::AuthPrincipal {
        client_id:   row.client_id,
        client_name: row.name,
        auth_method: common::AuthMethod::ClientCredentials,
    })
}

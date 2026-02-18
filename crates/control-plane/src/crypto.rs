//! Simple AES-256-GCM encryption for connector passwords.
//! Key derived from ENCRYPTION_KEY env var (32 bytes hex or generated on first run).

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Result, anyhow};
use rand::Rng;

pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    /// Initialize from ENCRYPTION_KEY env var (64 hex chars = 32 bytes).
    /// If not set, generates a random key and warns (NOT production-safe).
    pub fn new() -> Result<Self> {
        let key_hex = std::env::var("ENCRYPTION_KEY").unwrap_or_else(|_| {
            let random_key: [u8; 32] = rand::thread_rng().gen();
            let hex = hex::encode(random_key);
            tracing::warn!(
                "⚠️  ENCRYPTION_KEY not set — using random key (data will be lost on restart!)\n\
                 Set ENCRYPTION_KEY={} in production",
                hex
            );
            hex
        });

        let key_bytes = hex::decode(&key_hex)
            .map_err(|_| anyhow!("ENCRYPTION_KEY must be 64 hex characters (32 bytes)"))?;

        if key_bytes.len() != 32 {
            return Err(anyhow!("ENCRYPTION_KEY must be exactly 32 bytes (64 hex chars)"));
        }

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Encrypt plaintext. Returns base64(nonce || ciphertext).
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(base64::encode(combined))
    }

    /// Decrypt base64(nonce || ciphertext).
    pub fn decrypt(&self, encrypted_b64: &str) -> Result<String> {
        let combined = base64::decode(encrypted_b64)
            .map_err(|_| anyhow!("Invalid base64"))?;

        if combined.len() < 12 {
            return Err(anyhow!("Encrypted data too short"));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|_| anyhow!("Decrypted data is not valid UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        std::env::set_var("ENCRYPTION_KEY", "a".repeat(64));
        let crypto = CryptoService::new().unwrap();

        let plaintext = "my-secret-password";
        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}

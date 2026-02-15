use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use std::env;

pub struct Cipher {
    key: chacha20poly1305::Key,
}

impl Cipher {
    pub fn new() -> Result<Self, String> {
        let key_hex = env::var("ENCRYPTION_KEY").map_err(|_| {
            "ENCRYPTION_KEY not set in environment".to_string()
        })?;

        let key_bytes = hex::decode(&key_hex)
            .map_err(|e| format!("Failed to decode ENCRYPTION_KEY: {}", e))?;

        if key_bytes.len() != 32 {
            return Err("ENCRYPTION_KEY must be 32 bytes (64 hex chars)".to_string());
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);

        Ok(Cipher {
            key: chacha20poly1305::Key::from(key_array),
        })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let cipher = ChaCha20Poly1305::new(&self.key);
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = cipher
            .encrypt(&nonce, Payload::from(plaintext.as_bytes()))
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut encrypted = Vec::new();
        encrypted.extend_from_slice(&nonce_bytes);
        encrypted.extend_from_slice(&ciphertext);

        Ok(hex::encode(encrypted))
    }

    pub fn decrypt(&self, encrypted_hex: &str) -> Result<String, String> {
        let encrypted = hex::decode(encrypted_hex)
            .map_err(|e| format!("Failed to decode encrypted data: {}", e))?;

        if encrypted.len() < 12 {
            return Err("Invalid encrypted data: too short".to_string());
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = ChaCha20Poly1305::new(&self.key);
        let plaintext = cipher
            .decrypt(nonce, Payload::from(ciphertext))
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|e| format!("Failed to convert decrypted data to UTF-8: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        env::set_var("ENCRYPTION_KEY", "0".repeat(64));
        let cipher = Cipher::new().unwrap();
        let plaintext = "my-secret-api-key-12345";

        let encrypted = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
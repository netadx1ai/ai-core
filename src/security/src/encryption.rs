//! Encryption Services Module
//!
//! Provides comprehensive encryption services for the AI-CORE security framework.
//! Supports AES-256-GCM, ChaCha20-Poly1305, key management, and password hashing.

use crate::constants::{AES_KEY_SIZE, CHACHA20_KEY_SIZE};
use crate::errors::{SecurityError, SecurityResult};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chacha20poly1305::{
    aead::{Aead as ChaChaAead, KeyInit as ChaChaKeyInit, OsRng as ChaChaOsRng},
    ChaCha20Poly1305, Key as ChaChaKey, Nonce as ChaChaNonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use pbkdf2::pbkdf2_hmac;
use ring::hmac;
use sha2::{Digest, Sha256, Sha512};
use zeroize::{Zeroize, ZeroizeOnDrop};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Encryption algorithm identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM (recommended for most use cases)
    Aes256Gcm,
    /// ChaCha20-Poly1305 (recommended for high-performance scenarios)
    ChaCha20Poly1305,
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        Self::Aes256Gcm
    }
}

/// Key derivation function identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyDerivationFunction {
    /// PBKDF2 with HMAC-SHA256
    Pbkdf2HmacSha256,
    /// Scrypt (memory-hard function)
    Scrypt,
    /// Argon2id (recommended for password hashing)
    Argon2id,
    /// HKDF with HMAC-SHA256
    HkdfSha256,
}

/// Encrypted data container with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted ciphertext
    pub ciphertext: String,
    /// Nonce/IV used for encryption
    pub nonce: String,
    /// Algorithm used for encryption
    pub algorithm: EncryptionAlgorithm,
    /// Key identifier for key rotation
    pub key_id: String,
    /// Encryption timestamp
    pub encrypted_at: DateTime<Utc>,
    /// Associated authenticated data
    pub associated_data: Option<String>,
}

/// Cryptographic key with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    /// Unique key identifier
    pub id: String,
    /// Key material (zeroized on drop)
    pub key: Vec<u8>,
    /// Algorithm this key is intended for
    pub algorithm: EncryptionAlgorithm,
    /// Key creation timestamp
    pub created_at: DateTime<Utc>,
    /// Key expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Key generation counter
    pub generation: u32,
    /// Key purpose
    pub purpose: KeyPurpose,
    /// Key derivation parameters (if derived)
    pub derivation: Option<KeyDerivationParams>,
}

impl Zeroize for EncryptionKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

impl ZeroizeOnDrop for EncryptionKey {}

/// Key usage purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPurpose {
    /// Data encryption/decryption
    DataEncryption,
    /// Key encryption (KEK)
    KeyEncryption,
    /// Message authentication
    Authentication,
    /// Digital signatures
    Signing,
    /// Key derivation
    Derivation,
}

/// Password configuration
#[derive(Debug, Clone)]
pub struct PasswordConfig {
    pub memory_cost: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub hash_length: u32,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            memory_cost: 65536, // 64 MB
            time_cost: 3,
            parallelism: 1,
            hash_length: 32,
        }
    }
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationParams {
    pub function: KeyDerivationFunction,
    pub iterations: u32,
    pub salt_length: usize,
    pub output_length: usize,
}

impl Default for KeyDerivationParams {
    fn default() -> Self {
        Self {
            function: KeyDerivationFunction::Pbkdf2HmacSha256,
            iterations: 100_000,
            salt_length: 32,
            output_length: 32,
        }
    }
}

/// Password hash result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHashResult {
    /// Hash string (PHC format)
    pub hash: String,
    /// Salt used for hashing
    pub salt: Vec<u8>,
    /// Algorithm identifier
    pub algorithm: String,
    /// Hash creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Digital signature container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalSignature {
    /// Signature bytes
    pub signature: Vec<u8>,
    /// Public key for verification
    pub public_key: Vec<u8>,
    /// Algorithm used
    pub algorithm: String,
    /// Signature timestamp
    pub signed_at: DateTime<Utc>,
}

/// Password strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrengthLevel {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

/// Key manager trait for dependency injection
#[async_trait]
pub trait KeyManager: Send + Sync {
    async fn generate_key(
        &self,
        algorithm: EncryptionAlgorithm,
        purpose: KeyPurpose,
    ) -> SecurityResult<String>;
    async fn get_key(&self, key_id: &str) -> SecurityResult<EncryptionKey>;
    async fn get_default_key(&self) -> SecurityResult<EncryptionKey>;
    async fn rotate_key(&self, old_key_id: &str) -> SecurityResult<String>;
    async fn list_keys(&self) -> SecurityResult<Vec<String>>;
    async fn cleanup_expired_keys(&self) -> SecurityResult<u32>;
    async fn derive_key_from_password(
        &self,
        password: &str,
        salt: &[u8],
        algorithm: EncryptionAlgorithm,
        kdf: KeyDerivationFunction,
    ) -> SecurityResult<String>;
}

/// In-memory key manager implementation
pub struct InMemoryKeyManager {
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    default_key_id: Arc<RwLock<Option<String>>>,
    rotation_interval: Duration,
}

impl InMemoryKeyManager {
    pub fn new(rotation_interval: Duration) -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            default_key_id: Arc::new(RwLock::new(None)),
            rotation_interval,
        }
    }

    pub async fn initialize_with_defaults(&self) -> SecurityResult<()> {
        // Generate default keys for each algorithm
        let aes_key_id = self
            .generate_key(EncryptionAlgorithm::Aes256Gcm, KeyPurpose::DataEncryption)
            .await?;
        let _chacha_key_id = self
            .generate_key(
                EncryptionAlgorithm::ChaCha20Poly1305,
                KeyPurpose::DataEncryption,
            )
            .await?;

        let mut default_key_id = self.default_key_id.write().await;
        *default_key_id = Some(aes_key_id);

        Ok(())
    }

    async fn generate_key_internal(
        &self,
        algorithm: EncryptionAlgorithm,
        purpose: KeyPurpose,
    ) -> SecurityResult<EncryptionKey> {
        let key_size = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => AES_KEY_SIZE,
            EncryptionAlgorithm::ChaCha20Poly1305 => CHACHA20_KEY_SIZE,
        };

        let mut key_data = vec![0u8; key_size];
        OsRng.fill_bytes(&mut key_data);

        let key_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        Ok(EncryptionKey {
            id: key_id,
            key: key_data,
            algorithm,
            created_at: now,
            expires_at: Some(now + self.rotation_interval),
            generation: 1,
            purpose,
            derivation: None,
        })
    }
}

#[async_trait]
impl KeyManager for InMemoryKeyManager {
    async fn generate_key(
        &self,
        algorithm: EncryptionAlgorithm,
        purpose: KeyPurpose,
    ) -> SecurityResult<String> {
        let key = self.generate_key_internal(algorithm, purpose).await?;
        let key_id = key.id.clone();

        let mut keys = self.keys.write().await;
        keys.insert(key_id.clone(), key);

        // Set as default if none exists
        let mut default_key_id = self.default_key_id.write().await;
        if default_key_id.is_none() {
            *default_key_id = Some(key_id.clone());
        }

        Ok(key_id)
    }

    async fn get_key(&self, key_id: &str) -> SecurityResult<EncryptionKey> {
        let keys = self.keys.read().await;
        keys.get(key_id)
            .cloned()
            .ok_or_else(|| SecurityError::KeyNotFound(key_id.to_string()))
    }

    async fn get_default_key(&self) -> SecurityResult<EncryptionKey> {
        let default_key_id = self.default_key_id.read().await;
        match default_key_id.as_ref() {
            Some(key_id) => self.get_key(key_id).await,
            None => Err(SecurityError::KeyNotFound("No default key set".to_string())),
        }
    }

    async fn rotate_key(&self, old_key_id: &str) -> SecurityResult<String> {
        let old_key = self.get_key(old_key_id).await?;
        let mut new_key = self
            .generate_key_internal(old_key.algorithm, old_key.purpose)
            .await?;
        new_key.generation = old_key.generation + 1;

        let new_key_id = new_key.id.clone();

        let mut keys = self.keys.write().await;
        keys.insert(new_key_id.clone(), new_key);

        Ok(new_key_id)
    }

    async fn list_keys(&self) -> SecurityResult<Vec<String>> {
        let keys = self.keys.read().await;
        Ok(keys.keys().cloned().collect())
    }

    async fn cleanup_expired_keys(&self) -> SecurityResult<u32> {
        let now = Utc::now();
        let mut keys = self.keys.write().await;
        let default_key_id = self.default_key_id.read().await;
        let mut removed_count = 0;

        keys.retain(|key_id, key| {
            if let Some(expires_at) = key.expires_at {
                if expires_at <= now && default_key_id.as_ref() != Some(key_id) {
                    removed_count += 1;
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        Ok(removed_count)
    }

    async fn derive_key_from_password(
        &self,
        password: &str,
        salt: &[u8],
        algorithm: EncryptionAlgorithm,
        kdf: KeyDerivationFunction,
    ) -> SecurityResult<String> {
        let key_size = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => AES_KEY_SIZE,
            EncryptionAlgorithm::ChaCha20Poly1305 => CHACHA20_KEY_SIZE,
        };

        let mut key_data = vec![0u8; key_size];

        match kdf {
            KeyDerivationFunction::Pbkdf2HmacSha256 => {
                pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key_data);
            }
            KeyDerivationFunction::Argon2id => {
                let argon2 = Argon2::default();
                let salt_string = argon2::password_hash::SaltString::encode_b64(salt)
                    .map_err(|e| SecurityError::KeyDerivation(e.to_string()))?;
                let hash = argon2
                    .hash_password(password.as_bytes(), &salt_string)
                    .map_err(|e| SecurityError::KeyDerivation(e.to_string()))?;
                key_data.copy_from_slice(&hash.hash.unwrap().as_bytes()[..key_size]);
            }
            _ => {
                return Err(SecurityError::UnsupportedOperation(
                    "KDF not implemented".to_string(),
                ))
            }
        }

        let key = EncryptionKey {
            id: Uuid::new_v4().to_string(),
            key: key_data,
            algorithm,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + self.rotation_interval),
            generation: 1,
            purpose: KeyPurpose::DataEncryption,
            derivation: Some(KeyDerivationParams {
                function: kdf,
                iterations: 100_000,
                salt_length: salt.len(),
                output_length: key_size,
            }),
        };

        let key_id = key.id.clone();
        let mut keys = self.keys.write().await;
        keys.insert(key_id.clone(), key);

        Ok(key_id)
    }
}

/// Password service for secure password hashing and verification
pub struct PasswordService {
    config: PasswordConfig,
}

impl PasswordService {
    pub fn new() -> Self {
        Self {
            config: PasswordConfig::default(),
        }
    }

    pub fn new_with_config(config: PasswordConfig) -> Self {
        Self { config }
    }

    /// Hash a password using Argon2id
    pub fn hash_password(&self, password: &str) -> SecurityResult<PasswordHashResult> {
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::PasswordHashingFailed(e.to_string()))?;

        Ok(PasswordHashResult {
            hash: password_hash.to_string(),
            salt: salt.as_str().as_bytes().to_vec(),
            algorithm: "argon2id".to_string(),
            created_at: Utc::now(),
        })
    }

    /// Verify a password against its hash
    pub fn verify_password(
        &self,
        password: &str,
        hash_result: &PasswordHashResult,
    ) -> SecurityResult<bool> {
        let parsed_hash = PasswordHash::new(&hash_result.hash)
            .map_err(|e| SecurityError::PasswordVerificationFailed(e.to_string()))?;

        let argon2 = Argon2::default();
        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Check password strength
    pub fn check_password_strength(&self, password: &str) -> PasswordStrengthLevel {
        let length = password.len();
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        let criteria_met = [has_lower, has_upper, has_digit, has_special]
            .iter()
            .filter(|&&x| x)
            .count();

        match (length, criteria_met) {
            (0..=5, _) => PasswordStrengthLevel::VeryWeak,
            (6..=8, 0..=1) => PasswordStrengthLevel::VeryWeak,
            (6..=8, 2) => PasswordStrengthLevel::Weak,
            (9..=11, 0..=2) => PasswordStrengthLevel::Weak,
            (9..=11, 3..=4) => PasswordStrengthLevel::Medium,
            (12..=15, 0..=2) => PasswordStrengthLevel::Medium,
            (12..=15, 3..=4) => PasswordStrengthLevel::Strong,
            (16.., 3..=4) => PasswordStrengthLevel::VeryStrong,
            _ => PasswordStrengthLevel::Medium,
        }
    }
}

/// Main encryption service
pub struct EncryptionService {
    pub key_manager: Box<dyn KeyManager>,
}

impl EncryptionService {
    pub async fn new(key_manager: impl KeyManager + 'static) -> SecurityResult<Self> {
        Ok(Self {
            key_manager: Box::new(key_manager),
        })
    }

    /// Encrypt data using the default key
    pub async fn encrypt(&self, plaintext: &[u8]) -> SecurityResult<EncryptedData> {
        let key = self.key_manager.get_default_key().await?;
        self.encrypt_with_key(plaintext, &key, None).await
    }

    /// Encrypt data with additional authenticated data
    pub async fn encrypt_with_aad(
        &self,
        plaintext: &[u8],
        aad: &[u8],
    ) -> SecurityResult<EncryptedData> {
        let key = self.key_manager.get_default_key().await?;
        self.encrypt_with_key(plaintext, &key, Some(aad)).await
    }

    /// Encrypt data with a specific key
    pub async fn encrypt_with_key(
        &self,
        plaintext: &[u8],
        key: &EncryptionKey,
        aad: Option<&[u8]>,
    ) -> SecurityResult<EncryptedData> {
        let (ciphertext, nonce) = match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&key.key));
                let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

                let ciphertext = if let Some(aad) = aad {
                    cipher
                        .encrypt(
                            &nonce,
                            aes_gcm::aead::Payload {
                                msg: plaintext,
                                aad,
                            },
                        )
                        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?
                } else {
                    cipher
                        .encrypt(&nonce, plaintext)
                        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?
                };
                (ciphertext, nonce.to_vec())
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(ChaChaKey::from_slice(&key.key));
                let nonce = ChaCha20Poly1305::generate_nonce(&mut ChaChaOsRng);

                let ciphertext = if let Some(aad) = aad {
                    cipher
                        .encrypt(
                            &nonce,
                            chacha20poly1305::aead::Payload {
                                msg: plaintext,
                                aad,
                            },
                        )
                        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?
                } else {
                    cipher
                        .encrypt(&nonce, plaintext)
                        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?
                };
                (ciphertext, nonce.to_vec())
            }
        };

        Ok(EncryptedData {
            ciphertext: BASE64_STANDARD.encode(&ciphertext),
            nonce: BASE64_STANDARD.encode(&nonce),
            algorithm: key.algorithm,
            key_id: key.id.clone(),
            encrypted_at: Utc::now(),
            associated_data: aad.map(|a| BASE64_STANDARD.encode(a)),
        })
    }

    /// Decrypt data
    pub async fn decrypt(&self, encrypted: &EncryptedData) -> SecurityResult<Vec<u8>> {
        let key = self.key_manager.get_key(&encrypted.key_id).await?;

        let ciphertext = BASE64_STANDARD
            .decode(&encrypted.ciphertext)
            .map_err(|e| SecurityError::InvalidInputFormat(e.to_string()))?;
        let nonce_bytes = BASE64_STANDARD
            .decode(&encrypted.nonce)
            .map_err(|e| SecurityError::InvalidInputFormat(e.to_string()))?;

        let plaintext = match encrypted.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&key.key));
                let nonce = Nonce::from_slice(&nonce_bytes);

                if let Some(ref aad_b64) = encrypted.associated_data {
                    let aad = BASE64_STANDARD
                        .decode(aad_b64)
                        .map_err(|e| SecurityError::InvalidInputFormat(e.to_string()))?;
                    cipher
                        .decrypt(
                            nonce,
                            aes_gcm::aead::Payload {
                                msg: &ciphertext,
                                aad: &aad,
                            },
                        )
                        .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?
                } else {
                    cipher
                        .decrypt(nonce, ciphertext.as_slice())
                        .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?
                }
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(ChaChaKey::from_slice(&key.key));
                let nonce = ChaChaNonce::from_slice(&nonce_bytes);

                if let Some(ref aad_b64) = encrypted.associated_data {
                    let aad = BASE64_STANDARD
                        .decode(aad_b64)
                        .map_err(|e| SecurityError::InvalidInputFormat(e.to_string()))?;
                    cipher
                        .decrypt(
                            nonce,
                            chacha20poly1305::aead::Payload {
                                msg: &ciphertext,
                                aad: &aad,
                            },
                        )
                        .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?
                } else {
                    cipher
                        .decrypt(nonce, ciphertext.as_slice())
                        .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?
                }
            }
        };

        Ok(plaintext)
    }

    /// Encrypt a string and return base64-encoded result
    pub async fn encrypt_string(&self, plaintext: &str) -> SecurityResult<String> {
        let encrypted = self.encrypt(plaintext.as_bytes()).await?;
        let serialized = serde_json::to_vec(&encrypted)
            .map_err(|e| SecurityError::SerializationFailed(e.to_string()))?;
        Ok(BASE64_STANDARD.encode(serialized))
    }

    /// Decrypt a base64-encoded encrypted string
    pub async fn decrypt_string(&self, encrypted_b64: &str) -> SecurityResult<String> {
        let serialized = BASE64_STANDARD
            .decode(encrypted_b64)
            .map_err(|e| SecurityError::DeserializationFailed(e.to_string()))?;
        let encrypted: EncryptedData = serde_json::from_slice(&serialized)
            .map_err(|e| SecurityError::DeserializationFailed(e.to_string()))?;
        let plaintext = self.decrypt(&encrypted).await?;
        String::from_utf8(plaintext)
            .map_err(|e| SecurityError::DeserializationFailed(e.to_string()))
    }

    /// Generate cryptographically secure random bytes
    pub fn generate_random_bytes(&self, length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut bytes);
        bytes
    }

    /// Sign a message using Ed25519
    pub fn sign_message(
        &self,
        message: &[u8],
        private_key: &[u8],
    ) -> SecurityResult<DigitalSignature> {
        if private_key.len() != 32 {
            return Err(SecurityError::InvalidKey(
                "Invalid private key length".to_string(),
            ));
        }

        let signing_key = SigningKey::from_bytes(private_key.try_into().unwrap());
        let verifying_key = signing_key.verifying_key();

        let signature = signing_key.sign(message);

        Ok(DigitalSignature {
            signature: signature.to_bytes().to_vec(),
            public_key: verifying_key.to_bytes().to_vec(),
            algorithm: "Ed25519".to_string(),
            signed_at: Utc::now(),
        })
    }

    /// Verify a digital signature
    pub fn verify_signature(
        &self,
        message: &[u8],
        signature: &DigitalSignature,
    ) -> SecurityResult<bool> {
        let verifying_key =
            VerifyingKey::from_bytes(&signature.public_key.clone().try_into().unwrap())
                .map_err(|e| SecurityError::InvalidKey(e.to_string()))?;

        let sig = Signature::from_bytes(&signature.signature.clone().try_into().unwrap());

        Ok(verifying_key.verify(message, &sig).is_ok())
    }

    /// Compute HMAC-SHA256
    pub fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> SecurityResult<Vec<u8>> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, key);
        let tag = hmac::sign(&key, data);
        Ok(tag.as_ref().to_vec())
    }

    /// Verify HMAC-SHA256
    pub fn verify_hmac(
        &self,
        key: &[u8],
        data: &[u8],
        expected_hmac: &[u8],
    ) -> SecurityResult<bool> {
        let computed_hmac = self.hmac_sha256(key, data)?;
        Ok(computed_hmac == expected_hmac)
    }

    /// Compute SHA-256 hash
    pub fn sha256(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Compute SHA-512 hash
    pub fn sha512(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha512::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_key_manager() {
        let key_manager = InMemoryKeyManager::new(Duration::days(30));

        // Generate a key
        let key_id = key_manager
            .generate_key(EncryptionAlgorithm::Aes256Gcm, KeyPurpose::DataEncryption)
            .await
            .unwrap();

        // Retrieve the key
        let key = key_manager.get_key(&key_id).await.unwrap();
        assert_eq!(key.id, key_id);
        assert_eq!(key.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert_eq!(key.key.len(), AES_KEY_SIZE);

        // Test default key
        let default_key = key_manager.get_default_key().await.unwrap();
        assert_eq!(default_key.id, key_id);
    }

    #[tokio::test]
    async fn test_password_service() {
        let password_service = PasswordService::new();
        let password = "secure_password_123!";

        // Hash password
        let hash_result = password_service.hash_password(password).unwrap();
        assert!(!hash_result.hash.is_empty());
        assert_eq!(hash_result.algorithm, "argon2id");

        // Verify correct password
        let is_valid = password_service
            .verify_password(password, &hash_result)
            .unwrap();
        assert!(is_valid);

        // Verify incorrect password
        let is_invalid = password_service
            .verify_password("wrong_password", &hash_result)
            .unwrap();
        assert!(!is_invalid);
    }

    #[tokio::test]
    async fn test_encryption_service() {
        let key_manager = InMemoryKeyManager::new(Duration::days(30));
        let encryption_service = EncryptionService::new(key_manager).await.unwrap();

        // Generate a key for testing
        let _key_id = encryption_service
            .key_manager
            .generate_key(EncryptionAlgorithm::Aes256Gcm, KeyPurpose::DataEncryption)
            .await
            .unwrap();

        let plaintext = b"Hello, secure world!";

        // Test encryption and decryption
        let encrypted = encryption_service.encrypt(plaintext).await.unwrap();
        assert_eq!(encrypted.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert!(!encrypted.ciphertext.is_empty());
        assert!(!encrypted.nonce.is_empty());

        let decrypted = encryption_service.decrypt(&encrypted).await.unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_string_encryption() {
        let key_manager = InMemoryKeyManager::new(Duration::days(30));
        let encryption_service = EncryptionService::new(key_manager).await.unwrap();

        let _key_id = encryption_service
            .key_manager
            .generate_key(EncryptionAlgorithm::Aes256Gcm, KeyPurpose::DataEncryption)
            .await
            .unwrap();

        let plaintext = "Hello, encrypted string!";

        let encrypted_b64 = encryption_service.encrypt_string(plaintext).await.unwrap();
        let decrypted = encryption_service
            .decrypt_string(&encrypted_b64)
            .await
            .unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_password_strength() {
        let password_service = PasswordService::new();

        assert_eq!(
            password_service.check_password_strength("weak"),
            PasswordStrengthLevel::VeryWeak
        );
        assert_eq!(
            password_service.check_password_strength("WeakPass"),
            PasswordStrengthLevel::Weak
        );
        assert_eq!(
            password_service.check_password_strength("StrongPass123!"),
            PasswordStrengthLevel::Strong
        );
        assert_eq!(
            password_service.check_password_strength("VeryStrongPassword123!@#"),
            PasswordStrengthLevel::VeryStrong
        );
    }
}

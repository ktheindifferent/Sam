use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::Aes256;
use aes_gcm::{aead::Aead, Aes256Gcm, Nonce};
use chrono::{DateTime, Utc};
use rand::{thread_rng, Rng};
use ring::pbkdf2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const AES_GCM_PREFIX: &[u8] = b"SAMGCM1";
const AES_GCM_NONCE_LEN: usize = 12;

/// Password entry in the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub title: String,
    pub username: String,
    pub encrypted_password: Vec<u8>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub password_strength: PasswordStrength,
    pub expiry_date: Option<DateTime<Utc>>,
}

/// Password strength levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

/// Password vault for secure storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordVault {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub entries: HashMap<String, PasswordEntry>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub salt: Vec<u8>,
    pub iterations: u32,
}

impl PasswordVault {
    /// Create a new password vault
    pub fn new(owner_id: String, name: String) -> Self {
        let mut salt = vec![0u8; 32];
        thread_rng().fill(&mut salt[..]);

        PasswordVault {
            id: uuid::Uuid::new_v4().to_string(),
            owner_id,
            name,
            entries: HashMap::new(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            salt,
            iterations: 100_000, // PBKDF2 iterations
        }
    }

    /// Derive encryption key from master password
    fn derive_key(&self, master_password: &str) -> Result<Vec<u8>, String> {
        let mut key = vec![0u8; 32];
        let iterations = std::num::NonZeroU32::new(self.iterations)
            .ok_or_else(|| "PBKDF2 iterations must be greater than zero".to_string())?;
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            iterations,
            &self.salt,
            master_password.as_bytes(),
            &mut key,
        );
        Ok(key)
    }

    /// Add a new password entry
    pub fn add_entry(
        &mut self,
        master_password: &str,
        title: String,
        username: String,
        password: String,
        url: Option<String>,
        notes: Option<String>,
        tags: Vec<String>,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let key = self.derive_key(master_password)?;

        // Encrypt the password
        let encrypted_password = encrypt_password(&password, &key)?;

        // Analyze password strength
        let password_strength = analyze_password_strength(&password);

        let entry = PasswordEntry {
            id: id.clone(),
            title,
            username,
            encrypted_password,
            url,
            notes,
            tags,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            last_accessed: None,
            password_strength,
            expiry_date: None,
        };

        self.entries.insert(id.clone(), entry);
        self.modified_at = Utc::now();

        Ok(id)
    }

    /// Get a password entry
    pub fn get_entry(
        &mut self,
        master_password: &str,
        entry_id: &str,
    ) -> Result<(PasswordEntry, String), String> {
        let key = self.derive_key(master_password)?;

        let entry = self
            .entries
            .get_mut(entry_id)
            .ok_or_else(|| "Entry not found".to_string())?;
        let password = decrypt_password(&entry.encrypted_password, &key)?;

        // Update last accessed time
        entry.last_accessed = Some(Utc::now());

        Ok((entry.clone(), password))
    }

    /// Update a password entry
    pub fn update_entry(
        &mut self,
        master_password: &str,
        entry_id: &str,
        new_password: Option<String>,
        new_title: Option<String>,
        new_username: Option<String>,
        new_url: Option<String>,
        new_notes: Option<String>,
        new_tags: Option<Vec<String>>,
    ) -> Result<(), String> {
        let key = if new_password.is_some() {
            Some(self.derive_key(master_password)?)
        } else {
            None
        };

        let entry = self
            .entries
            .get_mut(entry_id)
            .ok_or_else(|| "Entry not found".to_string())?;

        if let Some(password) = new_password {
            let key = key
                .as_ref()
                .ok_or_else(|| "Encryption key was not derived".to_string())?;
            entry.encrypted_password = encrypt_password(&password, key)?;
            entry.password_strength = analyze_password_strength(&password);
        }

        if let Some(title) = new_title {
            entry.title = title;
        }

        if let Some(username) = new_username {
            entry.username = username;
        }

        if let Some(url) = new_url {
            entry.url = Some(url);
        }

        if let Some(notes) = new_notes {
            entry.notes = Some(notes);
        }

        if let Some(tags) = new_tags {
            entry.tags = tags;
        }

        entry.modified_at = Utc::now();
        self.modified_at = Utc::now();

        Ok(())
    }

    /// Delete a password entry
    pub fn delete_entry(&mut self, entry_id: &str) -> Result<(), String> {
        self.entries
            .remove(entry_id)
            .ok_or_else(|| "Entry not found".to_string())?;

        self.modified_at = Utc::now();
        Ok(())
    }

    /// Search entries by title or tags
    pub fn search_entries(&self, query: &str) -> Vec<&PasswordEntry> {
        let query_lower = query.to_lowercase();

        self.entries
            .values()
            .filter(|entry| {
                entry.title.to_lowercase().contains(&query_lower)
                    || entry
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
                    || entry.username.to_lowercase().contains(&query_lower)
                    || entry
                        .url
                        .as_ref()
                        .is_some_and(|url| url.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Check for weak or duplicate passwords
    pub fn audit_passwords(&mut self, master_password: &str) -> PasswordAuditReport {
        let Ok(key) = self.derive_key(master_password) else {
            return PasswordAuditReport {
                total_passwords: self.entries.len(),
                weak_passwords: Vec::new(),
                duplicate_passwords: HashMap::new(),
                expired_passwords: Vec::new(),
                audit_date: Utc::now(),
            };
        };
        let mut weak_passwords = Vec::new();
        let mut duplicate_passwords = HashMap::new();
        let mut expired_passwords = Vec::new();
        let mut password_map: HashMap<String, Vec<String>> = HashMap::new();

        for (id, entry) in &self.entries {
            // Check password strength
            match entry.password_strength {
                PasswordStrength::VeryWeak | PasswordStrength::Weak => {
                    weak_passwords.push(id.clone());
                }
                _ => {}
            }

            // Check for expired passwords
            if let Some(expiry) = entry.expiry_date {
                if Utc::now() > expiry {
                    expired_passwords.push(id.clone());
                }
            }

            // Check for duplicates
            if let Ok(password) = decrypt_password(&entry.encrypted_password, &key) {
                password_map.entry(password).or_default().push(id.clone());
            }
        }

        // Find duplicates
        for (_, ids) in password_map {
            if ids.len() > 1 {
                for id in &ids {
                    duplicate_passwords
                        .entry(id.clone())
                        .or_insert_with(Vec::new)
                        .extend(ids.iter().filter(|&i| i != id).cloned());
                }
            }
        }

        PasswordAuditReport {
            total_passwords: self.entries.len(),
            weak_passwords,
            duplicate_passwords,
            expired_passwords,
            audit_date: Utc::now(),
        }
    }
}

/// Password audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordAuditReport {
    pub total_passwords: usize,
    pub weak_passwords: Vec<String>,
    pub duplicate_passwords: HashMap<String, Vec<String>>,
    pub expired_passwords: Vec<String>,
    pub audit_date: DateTime<Utc>,
}

/// Generate a secure random password
pub fn generate_password(
    length: usize,
    include_uppercase: bool,
    include_lowercase: bool,
    include_numbers: bool,
    include_symbols: bool,
) -> String {
    let mut charset = String::new();

    if include_lowercase {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if include_uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if include_numbers {
        charset.push_str("0123456789");
    }
    if include_symbols {
        charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
    }

    if charset.is_empty() {
        charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_string();
    }

    let chars: Vec<char> = charset.chars().collect();
    let mut rng = thread_rng();

    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// Analyze password strength
pub fn analyze_password_strength(password: &str) -> PasswordStrength {
    let length = password.len();
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());

    let variety_count =
        has_lowercase as usize + has_uppercase as usize + has_digit as usize + has_special as usize;

    // Check for common patterns
    let is_common_password = is_common_password(password);
    let has_sequence = contains_sequence(password);
    let has_repetition = contains_repetition(password);

    // Calculate strength
    if length < 6 {
        PasswordStrength::VeryWeak
    } else if is_common_password || length < 8 || variety_count < 2 || has_repetition {
        PasswordStrength::Weak
    } else if length < 10 || variety_count < 3 || (has_sequence && variety_count < 4) {
        PasswordStrength::Fair
    } else if length < 12 || variety_count < 4 || has_sequence {
        PasswordStrength::Strong
    } else {
        PasswordStrength::VeryStrong
    }
}

/// Check for commonly-used passwords.
fn is_common_password(password: &str) -> bool {
    let password_lower = password.to_ascii_lowercase();
    matches!(
        password_lower.as_str(),
        "password"
            | "password1"
            | "123456"
            | "12345678"
            | "123456789"
            | "qwerty"
            | "abc123"
            | "admin"
            | "letmein"
            | "welcome"
    )
}

/// Check for sequential characters
fn contains_sequence(password: &str) -> bool {
    let sequences = [
        "123", "234", "345", "456", "567", "678", "789", "890", "abc", "bcd", "cde", "def", "efg",
        "fgh", "ghi", "hij", "ijk", "jkl", "klm", "lmn", "mno", "nop", "opq", "pqr", "qrs", "rst",
        "stu", "tuv", "uvw", "vwx", "wxy", "xyz",
    ];

    let password_lower = password.to_lowercase();
    sequences.iter().any(|seq| password_lower.contains(seq))
}

/// Check for character repetition
fn contains_repetition(password: &str) -> bool {
    let chars: Vec<char> = password.chars().collect();
    for i in 0..chars.len().saturating_sub(2) {
        if chars[i] == chars[i + 1] && chars[i] == chars[i + 2] {
            return true;
        }
    }
    false
}

/// Encrypt a password using AES-256-GCM.
fn encrypt_password(password: &str, key: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("Invalid key length".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Cipher initialization error: {}", e))?;
    let mut nonce_bytes = [0u8; AES_GCM_NONCE_LEN];
    thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, password.as_bytes())
        .map_err(|e| format!("Encryption error: {}", e))?;

    let mut encrypted =
        Vec::with_capacity(AES_GCM_PREFIX.len() + AES_GCM_NONCE_LEN + ciphertext.len());
    encrypted.extend_from_slice(AES_GCM_PREFIX);
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend_from_slice(&ciphertext);

    Ok(encrypted)
}

/// Decrypt a password using AES-256
fn decrypt_password(encrypted: &[u8], key: &[u8]) -> Result<String, String> {
    if key.len() != 32 {
        return Err("Invalid key length".to_string());
    }

    if let Some(payload) = encrypted.strip_prefix(AES_GCM_PREFIX) {
        if payload.len() < AES_GCM_NONCE_LEN {
            return Err("Encrypted payload is too short".to_string());
        }

        let (nonce_bytes, ciphertext) = payload.split_at(AES_GCM_NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| format!("Cipher initialization error: {}", e))?;
        let decrypted = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|e| format!("Decryption error: {}", e))?;

        return String::from_utf8(decrypted).map_err(|e| format!("Decryption error: {}", e));
    }

    decrypt_legacy_password(encrypted, key)
}

/// Decrypt passwords written by the legacy AES-ECB format.
fn decrypt_legacy_password(encrypted: &[u8], key: &[u8]) -> Result<String, String> {
    if encrypted.len() % 16 != 0 {
        return Err("Invalid legacy encrypted payload length".to_string());
    }

    let key = GenericArray::from_slice(key);
    let cipher = Aes256::new(key);

    let mut decrypted = Vec::new();
    for chunk in encrypted.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        decrypted.extend_from_slice(&block);
    }

    // Remove padding
    while decrypted.last() == Some(&0) {
        decrypted.pop();
    }

    String::from_utf8(decrypted).map_err(|e| format!("Decryption error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_generation() {
        let password = generate_password(16, true, true, true, true);
        assert_eq!(password.len(), 16);

        // Test with only lowercase
        let password = generate_password(10, false, true, false, false);
        assert_eq!(password.len(), 10);
        assert!(password.chars().all(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn test_password_strength() {
        assert!(matches!(
            analyze_password_strength("12345"),
            PasswordStrength::VeryWeak
        ));

        assert!(matches!(
            analyze_password_strength("password"),
            PasswordStrength::Weak
        ));

        assert!(matches!(
            analyze_password_strength("Pass123"),
            PasswordStrength::Weak | PasswordStrength::Fair
        ));

        assert!(matches!(
            analyze_password_strength("MyP@ssw0rd123!"),
            PasswordStrength::Strong | PasswordStrength::VeryStrong
        ));
    }

    #[test]
    fn test_encryption_decryption() {
        let password = "mysecretpassword";
        let key = vec![1u8; 32];

        let encrypted = encrypt_password(password, &key).unwrap();
        let decrypted = decrypt_password(&encrypted, &key).unwrap();

        assert_eq!(password, decrypted);
    }

    #[test]
    fn test_encryption_uses_unique_nonce() {
        let password = "mysecretpassword";
        let key = vec![1u8; 32];

        let first = encrypt_password(password, &key).unwrap();
        let second = encrypt_password(password, &key).unwrap();

        assert_ne!(first, second);
        assert_eq!(decrypt_password(&first, &key).unwrap(), password);
        assert_eq!(decrypt_password(&second, &key).unwrap(), password);
    }

    #[test]
    fn test_zero_iterations_returns_error() {
        let mut vault = PasswordVault::new("user123".to_string(), "My Vault".to_string());
        vault.iterations = 0;

        let result = vault.add_entry(
            "master_password_123",
            "Gmail".to_string(),
            "user@gmail.com".to_string(),
            "mypassword123".to_string(),
            None,
            None,
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_vault_operations() {
        let mut vault = PasswordVault::new("user123".to_string(), "My Vault".to_string());
        let master_password = "master_password_123";

        // Add entry
        let id = vault
            .add_entry(
                master_password,
                "Gmail".to_string(),
                "user@gmail.com".to_string(),
                "mypassword123".to_string(),
                Some("https://gmail.com".to_string()),
                None,
                vec!["email".to_string()],
            )
            .unwrap();

        // Get entry
        let (entry, password) = vault.get_entry(master_password, &id).unwrap();
        assert_eq!(entry.title, "Gmail");
        assert_eq!(password, "mypassword123");

        // Search entries
        let results = vault.search_entries("gmail");
        assert_eq!(results.len(), 1);

        // Delete entry
        vault.delete_entry(&id).unwrap();
        assert_eq!(vault.entries.len(), 0);
    }
}

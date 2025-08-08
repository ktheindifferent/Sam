use aes::Aes256;
use aes::cipher::{
    BlockEncrypt, BlockDecrypt, KeyInit,
    generic_array::GenericArray,
};
use chrono::{DateTime, Utc};
use pbkdf2::{pbkdf2_hmac};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;

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
    fn derive_key(&self, master_password: &str) -> Vec<u8> {
        let mut key = vec![0u8; 32];
        pbkdf2_hmac::<Sha256>(
            master_password.as_bytes(),
            &self.salt,
            self.iterations,
            &mut key,
        );
        key
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
        let key = self.derive_key(master_password);
        
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
        let entry = self.entries.get_mut(entry_id)
            .ok_or_else(|| "Entry not found".to_string())?;
        
        let key = self.derive_key(master_password);
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
        let entry = self.entries.get_mut(entry_id)
            .ok_or_else(|| "Entry not found".to_string())?;
        
        if let Some(password) = new_password {
            let key = self.derive_key(master_password);
            entry.encrypted_password = encrypt_password(&password, &key)?;
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
        self.entries.remove(entry_id)
            .ok_or_else(|| "Entry not found".to_string())?;
        
        self.modified_at = Utc::now();
        Ok(())
    }
    
    /// Search entries by title or tags
    pub fn search_entries(&self, query: &str) -> Vec<&PasswordEntry> {
        let query_lower = query.to_lowercase();
        
        self.entries.values()
            .filter(|entry| {
                entry.title.to_lowercase().contains(&query_lower) ||
                entry.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower)) ||
                entry.username.to_lowercase().contains(&query_lower) ||
                entry.url.as_ref().map_or(false, |url| url.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
    
    /// Check for weak or duplicate passwords
    pub fn audit_passwords(&mut self, master_password: &str) -> PasswordAuditReport {
        let key = self.derive_key(master_password);
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
                password_map.entry(password)
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }
        }
        
        // Find duplicates
        for (_, ids) in password_map {
            if ids.len() > 1 {
                for id in &ids {
                    duplicate_passwords.entry(id.clone())
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
    
    let variety_count = has_lowercase as usize + has_uppercase as usize + 
                       has_digit as usize + has_special as usize;
    
    // Check for common patterns
    let has_sequence = contains_sequence(password);
    let has_repetition = contains_repetition(password);
    
    // Calculate strength
    if length < 6 || variety_count < 2 {
        PasswordStrength::VeryWeak
    } else if length < 8 || variety_count < 3 || has_sequence || has_repetition {
        PasswordStrength::Weak
    } else if length < 10 || variety_count < 3 {
        PasswordStrength::Fair
    } else if length < 12 || variety_count < 4 {
        PasswordStrength::Strong
    } else {
        PasswordStrength::VeryStrong
    }
}

/// Check for sequential characters
fn contains_sequence(password: &str) -> bool {
    let sequences = ["123", "234", "345", "456", "567", "678", "789", "890",
                     "abc", "bcd", "cde", "def", "efg", "fgh", "ghi", "hij",
                     "ijk", "jkl", "klm", "lmn", "mno", "nop", "opq", "pqr",
                     "qrs", "rst", "stu", "tuv", "uvw", "vwx", "wxy", "xyz"];
    
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

/// Encrypt a password using AES-256
fn encrypt_password(password: &str, key: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("Invalid key length".to_string());
    }
    
    let key = GenericArray::from_slice(key);
    let cipher = Aes256::new(key);
    
    // Pad password to block size (16 bytes)
    let mut padded = password.as_bytes().to_vec();
    while padded.len() % 16 != 0 {
        padded.push(0);
    }
    
    let mut encrypted = Vec::new();
    for chunk in padded.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        encrypted.extend_from_slice(&block);
    }
    
    Ok(encrypted)
}

/// Decrypt a password using AES-256
fn decrypt_password(encrypted: &[u8], key: &[u8]) -> Result<String, String> {
    if key.len() != 32 {
        return Err("Invalid key length".to_string());
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
    fn test_vault_operations() {
        let mut vault = PasswordVault::new("user123".to_string(), "My Vault".to_string());
        let master_password = "master_password_123";
        
        // Add entry
        let id = vault.add_entry(
            master_password,
            "Gmail".to_string(),
            "user@gmail.com".to_string(),
            "mypassword123".to_string(),
            Some("https://gmail.com".to_string()),
            None,
            vec!["email".to_string()],
        ).unwrap();
        
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
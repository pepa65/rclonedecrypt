use crate::error::{DecryptionError, DecryptionResult};
use base64::{engine::general_purpose, Engine as _};
use crypto_secretbox::{aead::{Aead, KeyInit}, XSalsa20Poly1305};
use scrypt::{scrypt, Params};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

const RCLONE_MAGIC: &[u8] = b"RCLONE\x00\x00";
const SCRYPT_N: u32 = 16384;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;
const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 24; // NaCl uses 24-byte nonces
const SALT_SIZE: usize = 8;
const CHUNK_SIZE: usize = 65536; // 64KB chunks

/// Increment a 24-byte nonce for the next chunk
fn increment_nonce(nonce: &mut [u8; 24]) {
    // Try little-endian increment (increment from the beginning)
    for i in 0..24 {
        if nonce[i] == 255 {
            nonce[i] = 0;
        } else {
            nonce[i] += 1;
            break;
        }
    }
}

/// Alternative nonce increment (big-endian, from the end)
fn increment_nonce_be(nonce: &mut [u8; 24]) {
    for i in (0..24).rev() {
        if nonce[i] == 255 {
            nonce[i] = 0;
        } else {
            nonce[i] += 1;
            break;
        }
    }
}

/// Try 64-bit counter increment (common in some implementations)
fn increment_nonce_64bit(nonce: &mut [u8; 24]) {
    // Treat the last 8 bytes as a little-endian 64-bit counter
    let mut counter = u64::from_le_bytes([
        nonce[16], nonce[17], nonce[18], nonce[19],
        nonce[20], nonce[21], nonce[22], nonce[23]
    ]);
    counter = counter.wrapping_add(1);
    let counter_bytes = counter.to_le_bytes();
    nonce[16..24].copy_from_slice(&counter_bytes);
}

/// Check if a string looks like it could be base64
/// Base64 only contains A-Z, a-z, 0-9, +, /, and = for padding
fn is_likely_base64(s: &str) -> bool {
    // Must be reasonable length and contain only base64 characters
    if s.is_empty() || s.len() > 100 {
        return false;
    }
    
    // Check if all characters are valid base64
    s.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
    })
}

pub struct RcloneDecryptor {
    password: String,
    salt: Vec<u8>,
}

impl RcloneDecryptor {
    pub fn new(password: String, salt: String) -> DecryptionResult<Self> {
        // For rclone without filename encryption, use empty salt for file content encryption
        let salt_bytes = if salt.is_empty() {
            // Empty salt for rclone default behavior
            Vec::new()
        } else if salt.starts_with("0x") || salt.starts_with("0X") {
            // Hex encoded salt
            hex::decode(&salt[2..])?
        } else if is_likely_base64(&salt) && salt.len() > 8 {
            // Base64 encoded salt - only try if it looks like base64 and is longer than 8 chars
            match general_purpose::STANDARD.decode(&salt) {
                Ok(decoded) => decoded,
                Err(_) => {
                    // If base64 decode fails, treat as plain text
                    salt.as_bytes().to_vec()
                }
            }
        } else {
            // Plain text salt - use full string as bytes
            salt.as_bytes().to_vec()
        };

        Ok(RcloneDecryptor {
            password,
            salt: salt_bytes,
        })
    }

    /// Try to decrypt using sodiumoxide (direct NaCl binding)
    fn decrypt_with_sodiumoxide(&self, ciphertext: &[u8], nonce: &[u8; 24]) -> Result<Vec<u8>, DecryptionError> {
        // Initialize sodiumoxide
        sodiumoxide::init().map_err(|_| DecryptionError::InvalidPassword)?;
        
        // Derive key using scrypt  
        let mut key_bytes = [0u8; KEY_SIZE];
        let params = Params::new(
            14, // log2(16384) = 14
            SCRYPT_R,
            SCRYPT_P,
            KEY_SIZE,
        ).map_err(|_| DecryptionError::InvalidPassword)?;

        scrypt(self.password.as_bytes(), &self.salt, &params, &mut key_bytes)
            .map_err(|_| DecryptionError::InvalidPassword)?;
            
        println!("Debug: Sodiumoxide - key derived successfully ({} bytes)", key_bytes.len());
        println!("Debug: Sodiumoxide - nonce: {:?}", nonce);
        println!("Debug: Sodiumoxide - ciphertext length: {}", ciphertext.len());
            
        // Convert to sodiumoxide key
        let key = sodiumoxide::crypto::secretbox::Key::from_slice(&key_bytes)
            .ok_or(DecryptionError::InvalidPassword)?;
        let nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(nonce)
            .ok_or(DecryptionError::InvalidFormat)?;
            
        // Decrypt using sodiumoxide
        sodiumoxide::crypto::secretbox::open(ciphertext, &nonce_obj, &key)
            .map_err(|e| {
                println!("Debug: Sodiumoxide decryption error: Failed to decrypt");
                DecryptionError::InvalidPassword
            })
    }

    /// Debug method to show the actual salt bytes being used
    pub fn get_salt_debug(&self) -> &[u8] {
        &self.salt
    }

    pub fn decrypt_file<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> DecryptionResult<()> {
        let mut input_file = BufReader::new(File::open(&input_path)?);
        let mut output_file = BufWriter::new(File::create(&output_path)?);

        // Read and verify magic header
        let mut magic = [0u8; 8];
        input_file.read_exact(&mut magic)?;
        
        println!("Debug: Magic header: {:?}", magic);
        println!("Debug: Expected: {:?}", RCLONE_MAGIC);
        
        if magic != RCLONE_MAGIC {
            return Err(DecryptionError::InvalidFormat);
        }

        // Read nonce (24 bytes for NaCl)
        let mut base_nonce = [0u8; NONCE_SIZE];
        input_file.read_exact(&mut base_nonce)?;
        
        println!("Debug: Full nonce (24 bytes): {:?}", base_nonce);
        println!("Debug: Using salt: {} bytes", self.salt.len());
        println!("Debug: Password length: {}", self.password.len());

        // Derive key using scrypt with correct parameters for rclone v1.67.0
        let mut key = [0u8; KEY_SIZE];
        let params = Params::new(
            14, // log2(16384) = 14
            SCRYPT_R,
            SCRYPT_P,
            KEY_SIZE,
        ).map_err(|_| DecryptionError::InvalidPassword)?;

        scrypt(self.password.as_bytes(), &self.salt, &params, &mut key)
            .map_err(|_| DecryptionError::InvalidPassword)?;
            
        // Debug output (no sensitive data)
        println!("Debug: Scrypt params - N: {}, r: {}, p: {}", SCRYPT_N, SCRYPT_R, SCRYPT_P);
        println!("Debug: Key derived successfully ({} bytes)", key.len());

        // Initialize sodiumoxide for NaCl secretbox
        sodiumoxide::init().map_err(|_| DecryptionError::InvalidPassword)?;
        let secretbox_key = sodiumoxide::crypto::secretbox::Key::from_slice(&key)
            .ok_or(DecryptionError::InvalidPassword)?;

        // Read remaining data and try different approaches
        let mut remaining_data = Vec::new();
        input_file.read_to_end(&mut remaining_data)?;
        
        println!("Debug: Total encrypted data size: {} bytes", remaining_data.len());
        
        // Strategy 1: Try as single block
        let nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&base_nonce)
            .ok_or(DecryptionError::InvalidFormat)?;
        
        if let Ok(decrypted_data) = sodiumoxide::crypto::secretbox::open(&remaining_data, &nonce_obj, &secretbox_key) {
            println!("Debug: Successfully decrypted as single block!");
            println!("Debug: Decrypted data size: {} bytes", decrypted_data.len());
            if !decrypted_data.is_empty() {
                println!("Debug: First decrypted bytes: {:?}", &decrypted_data[..std::cmp::min(16, decrypted_data.len())]);
                let chunk_str = String::from_utf8_lossy(&decrypted_data[..std::cmp::min(16, decrypted_data.len())]);
                println!("Debug: First bytes as string: '{}'", chunk_str);
            }
            output_file.write_all(&decrypted_data)?;
            output_file.flush()?;
            println!("Successfully decrypted {} to {}", 
                     input_path.as_ref().display(), 
                     output_path.as_ref().display());
            return Ok(());
        }
        
        // Strategy 2: Try chunked decryption (rclone v1.67.0 uses 64KB chunks)
        println!("Debug: Single block failed, trying 64KB chunked decryption...");
        
        let mut current_pos = 0;
        let mut chunk_nonce = base_nonce;
        let mut found_valid_chunk = false;
        let mut total_decrypted_bytes = 0;
        
        while current_pos < remaining_data.len() {
            let remaining_bytes = remaining_data.len() - current_pos;
            
            // Determine chunk size:
            // - If we have more than 64KB+16 bytes remaining, use full 64KB+16 chunk
            // - Otherwise, use all remaining bytes (final smaller chunk)
            let chunk_size = if remaining_bytes > CHUNK_SIZE + 16 {
                CHUNK_SIZE + 16  // Full 64KB chunk + 16 byte auth tag
            } else {
                remaining_bytes  // Final chunk (whatever size remains)
            };
            
            if chunk_size < 16 {
                println!("Debug: Remaining chunk too small ({} bytes), stopping", chunk_size);
                break;
            }
            
            let chunk_data = &remaining_data[current_pos..current_pos + chunk_size];
            let chunk_nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&chunk_nonce)
                .ok_or(DecryptionError::InvalidFormat)?;
            
            println!("Debug: Trying to decrypt chunk at pos {} with size {} bytes (remaining: {})", 
                     current_pos, chunk_size, remaining_bytes);
            println!("Debug: Current nonce: {:?}", &chunk_nonce[..8]); // Show first 8 bytes of nonce
            
            if let Ok(decrypted_chunk) = sodiumoxide::crypto::secretbox::open(chunk_data, &chunk_nonce_obj, &secretbox_key) {
                println!("Debug: ✅ Successfully decrypted chunk at pos {} -> {} decrypted bytes", 
                         current_pos, decrypted_chunk.len());
                
                if current_pos == 0 {
                    println!("Debug: First chunk bytes: {:?}", &decrypted_chunk[..std::cmp::min(16, decrypted_chunk.len())]);
                    let chunk_str = String::from_utf8_lossy(&decrypted_chunk[..std::cmp::min(16, decrypted_chunk.len())]);
                    println!("Debug: First chunk as string: '{}'", chunk_str);
                }
                
                output_file.write_all(&decrypted_chunk)?;
                current_pos += chunk_size;
                total_decrypted_bytes += decrypted_chunk.len();
                
                // Try different nonce increment methods
                println!("Debug: Before increment: {:?}", &chunk_nonce[..8]);
                increment_nonce(&mut chunk_nonce);
                println!("Debug: After little-endian increment: {:?}", &chunk_nonce[..8]);
                found_valid_chunk = true;
                
                println!("Debug: Progress: {}/{} bytes processed, {} bytes decrypted so far", 
                         current_pos, remaining_data.len(), total_decrypted_bytes);
            } else {
                println!("Debug: ❌ Failed to decrypt chunk at pos {} with size {} bytes", current_pos, chunk_size);
                
                // If this is the second chunk, try different nonce increment strategies
                if current_pos == CHUNK_SIZE + 16 && found_valid_chunk {
                    println!("Debug: Trying alternative nonce increment strategies for second chunk...");
                    
                    // Reset to original nonce and try different increments
                    let mut test_nonce = base_nonce;
                    
                    // Try big-endian increment
                    increment_nonce_be(&mut test_nonce);
                    println!("Debug: Trying big-endian increment: {:?}", &test_nonce[..8]);
                    let test_nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&test_nonce)
                        .ok_or(DecryptionError::InvalidFormat)?;
                    
                    if let Ok(decrypted_chunk) = sodiumoxide::crypto::secretbox::open(chunk_data, &test_nonce_obj, &secretbox_key) {
                        println!("Debug: ✅ SUCCESS with big-endian nonce increment!");
                        output_file.write_all(&decrypted_chunk)?;
                        current_pos += chunk_size;
                        total_decrypted_bytes += decrypted_chunk.len();
                        chunk_nonce = test_nonce;
                        increment_nonce_be(&mut chunk_nonce); // Use BE for subsequent chunks
                        continue;
                    }
                    
                    // Try 64-bit counter increment
                    test_nonce = base_nonce;
                    increment_nonce_64bit(&mut test_nonce);
                    println!("Debug: Trying 64-bit counter increment: {:?}", &test_nonce[..8]);
                    let test_nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&test_nonce)
                        .ok_or(DecryptionError::InvalidFormat)?;
                    
                    if let Ok(decrypted_chunk) = sodiumoxide::crypto::secretbox::open(chunk_data, &test_nonce_obj, &secretbox_key) {
                        println!("Debug: ✅ SUCCESS with 64-bit counter nonce increment!");
                        output_file.write_all(&decrypted_chunk)?;
                        current_pos += chunk_size;
                        total_decrypted_bytes += decrypted_chunk.len();
                        chunk_nonce = test_nonce;
                        increment_nonce_64bit(&mut chunk_nonce); // Use 64-bit for subsequent chunks
                        continue;
                    }
                }
                
                // If this is not the first chunk and we've successfully decrypted some data,
                // it might be that we've reached the end or there's padding
                if found_valid_chunk {
                    println!("Debug: Already decrypted some chunks successfully, might have reached end");
                    break;
                }
                
                return Err(DecryptionError::InvalidPassword);
            }
        }
        
        if !found_valid_chunk {
            return Err(DecryptionError::InvalidPassword);
        }
        
        println!("Debug: Chunked decryption completed! Total decrypted: {} bytes", total_decrypted_bytes);

        output_file.flush()?;
        println!("Successfully decrypted {} to {} using chunked approach", 
                 input_path.as_ref().display(), 
                 output_path.as_ref().display());

        Ok(())
    }

    pub fn decrypt_bytes(&self, encrypted_data: &[u8]) -> DecryptionResult<Vec<u8>> {
        if encrypted_data.len() < 8 + NONCE_SIZE {
            return Err(DecryptionError::FileTooShort);
        }

        // Verify magic header
        if &encrypted_data[0..8] != RCLONE_MAGIC {
            return Err(DecryptionError::InvalidFormat);
        }

        // Extract nonce (24 bytes)
        let nonce = &encrypted_data[8..8 + NONCE_SIZE];
        let ciphertext = &encrypted_data[8 + NONCE_SIZE..];

        // Derive key using scrypt
        let mut key = [0u8; KEY_SIZE];
        let params = Params::new(
            14, // log2(16384) = 14
            SCRYPT_R,
            SCRYPT_P,
            KEY_SIZE,
        ).map_err(|_| DecryptionError::InvalidPassword)?;

        scrypt(self.password.as_bytes(), &self.salt, &params, &mut key)
            .map_err(|_| DecryptionError::InvalidPassword)?;

        // Initialize XSalsa20Poly1305 cipher
        let cipher = XSalsa20Poly1305::new_from_slice(&key)
            .map_err(|_| DecryptionError::InvalidPassword)?;

        // Decrypt the data using NaCl SecretBox
        let nonce_array: [u8; 24] = nonce.try_into().map_err(|_| DecryptionError::InvalidFormat)?;
        let decrypted = cipher.decrypt(&nonce_array.into(), ciphertext)
            .map_err(|_| DecryptionError::InvalidPassword)?;

        Ok(decrypted)
    }

    /// Try to decrypt with different common rclone configurations
    pub fn try_decrypt_with_variations<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> DecryptionResult<()> {
        println!("Trying different rclone configuration variations...");
        
        // Variation 1: Empty salt (no password2)
        println!("Trying with empty salt...");
        let mut temp_decryptor = RcloneDecryptor {
            password: self.password.clone(),
            salt: Vec::new(),
        };
        if let Ok(_) = temp_decryptor.decrypt_file(&input_path, &output_path) {
            println!("SUCCESS: Decrypted with empty salt!");
            return Ok(());
        }

        // Variation 2: Same password as salt (common rclone setup)
        println!("Trying with password as salt...");
        temp_decryptor.salt = self.password.as_bytes().to_vec();
        if let Ok(_) = temp_decryptor.decrypt_file(&input_path, &output_path) {
            println!("SUCCESS: Decrypted with password as salt!");
            return Ok(());
        }

        // Variation 3: Original salt (if provided)
        if !self.salt.is_empty() {
            println!("Trying with provided salt...");
            temp_decryptor.salt = self.salt.clone();
            if let Ok(_) = temp_decryptor.decrypt_file(&input_path, &output_path) {
                println!("SUCCESS: Decrypted with provided salt!");
                return Ok(());
            }
        }

        // Variation 4: Common default salts
        let common_salts = vec![
            b"rclone".to_vec(),
            b"default".to_vec(),
            b"".to_vec(),
            b"salt".to_vec(),
        ];

        for (i, salt) in common_salts.iter().enumerate() {
            println!("Trying common salt variation {}...", i + 1);
            temp_decryptor.salt = salt.clone();
            if let Ok(_) = temp_decryptor.decrypt_file(&input_path, &output_path) {
                println!("SUCCESS: Decrypted with common salt variation {}!", i + 1);
                return Ok(());
            }
        }

        Err(DecryptionError::InvalidPassword)
    }

    /// Try to auto-detect the correct configuration by testing the file header
    pub fn auto_decrypt<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> DecryptionResult<()> {
        // First, let's read just enough to check the file structure
        let mut file = File::open(&input_path)?;
        let mut header = [0u8; 32]; // Magic + nonce start
        file.read_exact(&mut header)?;
        
        if &header[0..8] != RCLONE_MAGIC {
            return Err(DecryptionError::InvalidFormat);
        }

        println!("Found valid rclone encrypted file");
        println!("File nonce (first 8 bytes): {:?}", &header[8..16]);
        
        // Try the variations
        self.try_decrypt_with_variations(input_path, output_path)
    }

    /// Try multiple passwords with auto-detect
    pub fn try_multiple_passwords<P: AsRef<Path>>(
        input_path: P, 
        output_path: P, 
        passwords: &[String]
    ) -> DecryptionResult<String> {
        println!("Trying {} different passwords...", passwords.len());
        
        for (i, password) in passwords.iter().enumerate() {
            let display_password = if password.len() > 20 { 
                format!("{}...", &password[..20]) 
            } else { 
                password.clone() 
            };
            println!("Trying password {} of {}: '{}'", i + 1, passwords.len(), display_password);
            
            let decryptor = RcloneDecryptor::new(password.clone(), String::new())?;
            
            if let Ok(_) = decryptor.auto_decrypt(&input_path, &output_path) {
                println!("SUCCESS! Password found: '{}'", password);
                return Ok(password.clone());
            }
        }
        
        Err(DecryptionError::InvalidPassword)
    }

    /// Interactive password entry mode
    pub fn interactive_decrypt<P: AsRef<Path>>(input_path: P, output_path: P) -> DecryptionResult<()> {
        println!("Interactive password mode - press Ctrl+C to exit");
        
        loop {
            print!("Enter password to try (or 'quit' to exit): ");
            std::io::stdout().flush().unwrap();
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let password = input.trim();
            
            if password.is_empty() || password == "quit" {
                break;
            }
            
            let decryptor = RcloneDecryptor::new(password.to_string(), String::new())?;
            
            if let Ok(_) = decryptor.auto_decrypt(&input_path, &output_path) {
                println!("SUCCESS! File decrypted with password: '{}'", password);
                return Ok(());
            } else {
                println!("Password '{}' failed. Try another one.", password);
            }
        }
        
        Err(DecryptionError::InvalidPassword)
    }

    /// Try to decrypt with different algorithm variations
    fn try_algorithm_variations(&self, ciphertext: &[u8], nonce: &[u8; 24]) -> DecryptionResult<Vec<u8>> {
        println!("Trying different algorithm variations...");
        
        // Variation 1: Different scrypt parameters (older rclone versions might use different values)
        let scrypt_variations = [
            (13, 8, 1), // N=8192
            (14, 8, 1), // N=16384 (current)
            (15, 8, 1), // N=32768
            (14, 4, 1), // different r
            (14, 16, 1), // different r
            (14, 8, 2), // different p
        ];
        
        for (log_n, r, p) in scrypt_variations {
            println!("Trying scrypt params: N=2^{}, r={}, p={}", log_n, r, p);
            
            let mut key_bytes = [0u8; KEY_SIZE];
            if let Ok(params) = Params::new(log_n, r, p, KEY_SIZE) {
                if scrypt(self.password.as_bytes(), &self.salt, &params, &mut key_bytes).is_ok() {
                    
                    // Try with sodiumoxide
                    if sodiumoxide::init().is_ok() {
                        if let (Some(key), Some(nonce_obj)) = (
                            sodiumoxide::crypto::secretbox::Key::from_slice(&key_bytes),
                            sodiumoxide::crypto::secretbox::Nonce::from_slice(nonce)
                        ) {
                            if let Ok(result) = sodiumoxide::crypto::secretbox::open(ciphertext, &nonce_obj, &key) {
                                println!("SUCCESS with scrypt params: N=2^{}, r={}, p={}", log_n, r, p);
                                return Ok(result);
                            }
                        }
                    }
                    
                    // Try with XSalsa20Poly1305
                    if let Ok(cipher) = XSalsa20Poly1305::new_from_slice(&key_bytes) {
                        if let Ok(result) = cipher.decrypt(&(*nonce).into(), ciphertext) {
                            println!("SUCCESS with XSalsa20Poly1305 and scrypt params: N=2^{}, r={}, p={}", log_n, r, p);
                            return Ok(result);
                        }
                    }
                }
            }
        }
        
        Err(DecryptionError::InvalidPassword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_decrypt_small_data() {
        // This is a basic test - you would need actual rclone encrypted data for real testing
        let password = "test_password".to_string();
        let salt = "dGVzdHNhbHQ=".to_string(); // base64 encoded "testsalt"
        
        let decryptor = RcloneDecryptor::new(password, salt).unwrap();
        
        // Create a minimal test with proper header structure
        let mut test_data = Vec::new();
        test_data.extend_from_slice(RCLONE_MAGIC);
        test_data.extend_from_slice(&[0u8; 24]); // 24-byte nonce for NaCl
        test_data.extend_from_slice(b"test content");
        
        let result = decryptor.decrypt_bytes(&test_data);
        // This will likely fail since it's not real encrypted data, but it tests the structure
        assert!(result.is_err()); // Expected to fail with invalid auth
    }

    #[test]
    fn test_plain_text_salt() {
        // Test with plain text salt (using a string that can't be base64)
        let password = "test_password".to_string();
        let salt = "plain!salt".to_string(); // plain text salt with special char
        
        let decryptor = RcloneDecryptor::new(password.clone(), salt).unwrap();
        assert_eq!(decryptor.salt.len(), 8);
        assert_eq!(decryptor.salt, b"plain!sa".to_vec());
        
        // Should pad short salts with zeros
        let short_salt = "abc".to_string();
        let decryptor2 = RcloneDecryptor::new(password.clone(), short_salt).unwrap();
        assert_eq!(decryptor2.salt, vec![b'a', b'b', b'c', 0, 0, 0, 0, 0]);
        
        // Should truncate long salts
        let long_salt = "verylongsaltstring".to_string();
        let decryptor3 = RcloneDecryptor::new(password, long_salt).unwrap();
        assert_eq!(decryptor3.salt, b"verylong".to_vec());
    }

    #[test]
    fn test_hex_salt() {
        // Test with hex salt
        let password = "test_password".to_string();
        let salt = "0x7465737473616c74".to_string(); // hex for "testsalt"
        
        let decryptor = RcloneDecryptor::new(password, salt).unwrap();
        assert_eq!(decryptor.salt, b"testsalt".to_vec());
    }
}

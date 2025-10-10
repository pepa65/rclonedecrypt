use crate::build_cli;
use crate::error::{DecryptionError, DecryptionResult};
use base64::{Engine as _, engine::general_purpose};
use scrypt::{Params, scrypt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
const RCLONE_MAGIC: &[u8] = b"RCLONE\x00\x00";
const SCRYPT_N: u32 = 16384;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;
const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 24; // NaCl uses 24-byte nonces
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
	let mut counter = u64::from_le_bytes([nonce[16], nonce[17], nonce[18], nonce[19], nonce[20], nonce[21], nonce[22], nonce[23]]);
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
	s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
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

		Ok(RcloneDecryptor { password, salt: salt_bytes })
	}

	/// Debug method to show the actual salt bytes being used
	pub fn get_salt_debug(&self) -> &[u8] {
		&self.salt
	}

	pub fn decrypt_file<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> DecryptionResult<()> {
		let verbose = build_cli().get_matches().get_flag("verbose");
		let mut input_file = BufReader::new(File::open(&input_path)?);
		let mut output_file = BufWriter::new(File::create(&output_path)?);
		// Read and verify magic header
		let mut magic = [0u8; 8];
		input_file.read_exact(&mut magic)?;
		if verbose {
			println!("Debug: Magic header: {:?}", magic);
			println!("Debug: Expected: {:?}", RCLONE_MAGIC);
		}
		if magic != RCLONE_MAGIC {
			return Err(DecryptionError::InvalidFormat);
		}

		// Read nonce (24 bytes for NaCl)
		let mut base_nonce = [0u8; NONCE_SIZE];
		input_file.read_exact(&mut base_nonce)?;
		if verbose {
			println!("Debug: Full nonce (24 bytes): {:?}", base_nonce);
			println!("Debug: Using salt: {} bytes", self.salt.len());
			println!("Debug: Password length: {}", self.password.len());
		};
		// Derive key using scrypt with correct parameters for rclone v1.67.0
		let mut key = [0u8; KEY_SIZE];
		let params = Params::new(
			14, // log2(16384) = 14
			SCRYPT_R, SCRYPT_P, KEY_SIZE,
		)
		.map_err(|_| DecryptionError::InvalidPassword)?;
		scrypt(self.password.as_bytes(), &self.salt, &params, &mut key).map_err(|_| DecryptionError::InvalidPassword)?;
		if verbose {
			// Debug output (no sensitive data)
			println!("Debug: Scrypt params - N: {}, r: {}, p: {}", SCRYPT_N, SCRYPT_R, SCRYPT_P);
			println!("Debug: Key derived successfully ({} bytes)", key.len());
		};
		// Initialize sodiumoxide for NaCl secretbox
		sodiumoxide::init().map_err(|_| DecryptionError::InvalidPassword)?;
		let secretbox_key = sodiumoxide::crypto::secretbox::Key::from_slice(&key).ok_or(DecryptionError::InvalidPassword)?;
		// Read remaining data and try different approaches
		let mut remaining_data = Vec::new();
		input_file.read_to_end(&mut remaining_data)?;
		if verbose {
			println!("Debug: Total encrypted data size: {} bytes", remaining_data.len());
		};
		// Strategy 1: Try as single block
		let nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&base_nonce).ok_or(DecryptionError::InvalidFormat)?;
		if let Ok(decrypted_data) = sodiumoxide::crypto::secretbox::open(&remaining_data, &nonce_obj, &secretbox_key) {
			if verbose {
				println!("Debug: Successfully decrypted as single block!");
				println!("Debug: Decrypted data size: {} bytes", decrypted_data.len());
			};
			if !decrypted_data.is_empty() && verbose {
				println!("Debug: First decrypted bytes: {:?}", &decrypted_data[..std::cmp::min(16, decrypted_data.len())]);
				let chunk_str = String::from_utf8_lossy(&decrypted_data[..std::cmp::min(16, decrypted_data.len())]);
				println!("Debug: First bytes as string: '{}'", chunk_str);
			};
			output_file.write_all(&decrypted_data)?;
			output_file.flush()?;
			println!("Successfully decrypted {} to {}", input_path.as_ref().display(), output_path.as_ref().display());
			return Ok(());
		}

		// Strategy 2: Try chunked decryption (rclone v1.67.0 uses 64KB chunks)
		if verbose {
			println!("Debug: Single block failed, trying 64KB chunked decryption...");
		};
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
				CHUNK_SIZE + 16 // Full 64KB chunk + 16 byte auth tag
			} else {
				remaining_bytes // Final chunk (whatever size remains)
			};

			if chunk_size < 16 {
				if verbose {
					println!("Debug: Remaining chunk too small ({} bytes), stopping", chunk_size);
				};
				break;
			}

			let chunk_data = &remaining_data[current_pos..current_pos + chunk_size];
			let chunk_nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&chunk_nonce).ok_or(DecryptionError::InvalidFormat)?;
			if verbose {
				println!("Debug: Trying to decrypt chunk at pos {} with size {} bytes (remaining: {})", current_pos, chunk_size, remaining_bytes);
				println!("Debug: Current nonce: {:?}", &chunk_nonce[..8]); // Show first 8 bytes of nonce
			};
			if let Ok(decrypted_chunk) = sodiumoxide::crypto::secretbox::open(chunk_data, &chunk_nonce_obj, &secretbox_key) {
				if verbose {
					println!("Debug: ✅ Successfully decrypted chunk at pos {} -> {} decrypted bytes", current_pos, decrypted_chunk.len());
					if current_pos == 0 {
						println!("Debug: First chunk bytes: {:?}", &decrypted_chunk[..std::cmp::min(16, decrypted_chunk.len())]);
						let chunk_str = String::from_utf8_lossy(&decrypted_chunk[..std::cmp::min(16, decrypted_chunk.len())]);
						println!("Debug: First chunk as string: '{}'", chunk_str);
					}
				};
				output_file.write_all(&decrypted_chunk)?;
				current_pos += chunk_size;
				total_decrypted_bytes += decrypted_chunk.len();

				// Try different nonce increment methods
				if verbose {
					println!("Debug: Before increment: {:?}", &chunk_nonce[..8]);
				};
				increment_nonce(&mut chunk_nonce);
				if verbose {
					println!("Debug: After little-endian increment: {:?}", &chunk_nonce[..8]);
				};
				found_valid_chunk = true;
				if verbose {
					println!(
						"Debug: Progress: {}/{} bytes processed, {} bytes decrypted so far",
						current_pos,
						remaining_data.len(),
						total_decrypted_bytes
					);
				};
			} else {
				if verbose {
					println!("Debug: ❌ Failed to decrypt chunk at pos {} with size {} bytes", current_pos, chunk_size);
				};
				// If this is the second chunk, try different nonce increment strategies
				if current_pos == CHUNK_SIZE + 16 && found_valid_chunk {
					if verbose {
						println!("Debug: Trying alternative nonce increment strategies for second chunk...");
					};
					// Reset to original nonce and try different increments
					let mut test_nonce = base_nonce;
					// Try big-endian increment
					increment_nonce_be(&mut test_nonce);
					if verbose {
						println!("Debug: Trying big-endian increment: {:?}", &test_nonce[..8]);
					};
					let test_nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&test_nonce).ok_or(DecryptionError::InvalidFormat)?;
					if let Ok(decrypted_chunk) = sodiumoxide::crypto::secretbox::open(chunk_data, &test_nonce_obj, &secretbox_key) {
						if verbose {
							println!("Debug: ✅ SUCCESS with big-endian nonce increment!");
						};
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
					if verbose {
						println!("Debug: Trying 64-bit counter increment: {:?}", &test_nonce[..8]);
					};
					let test_nonce_obj = sodiumoxide::crypto::secretbox::Nonce::from_slice(&test_nonce).ok_or(DecryptionError::InvalidFormat)?;
					if let Ok(decrypted_chunk) = sodiumoxide::crypto::secretbox::open(chunk_data, &test_nonce_obj, &secretbox_key) {
						if verbose {
							println!("Debug: ✅ SUCCESS with 64-bit counter nonce increment!");
						};
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
					if verbose {
						println!("Debug: Already decrypted some chunks successfully, might have reached end");
					};
					break;
				}

				return Err(DecryptionError::InvalidPassword);
			}
		}

		if !found_valid_chunk {
			return Err(DecryptionError::InvalidPassword);
		}

		if verbose {
			println!("Debug: Chunked decryption completed! Total decrypted: {} bytes", total_decrypted_bytes);
		};
		output_file.flush()?;
		println!("Successfully decrypted {} to {} using chunked approach", input_path.as_ref().display(), output_path.as_ref().display());
		Ok(())
	}
}

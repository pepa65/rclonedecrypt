# rclonedecrypt - Usage Examples
This document provides practical examples of how to use `rclonedecrypt` to decrypt files that were encrypted with rclone.
Unfortunately it seems that `rclone` changes its commandline interface, flags and even formats a fair bit.
**This is tested with rclone v1.59**

## Getting Started
1. **Rclone-Encrypted File**
  Rclone creates `.bin` files when encryption is enabled with filename encryption disabled, or random names without extension.
  These files contain the encrypted data in the rclone encryption format.
2. **Encryption Settings**
  You'll need two pieces of information:
  - **Password**: The encryption password used with rclone
  - **Salt**: The salt (password2) used by rclone (usually stored in `rclone.conf`)

```bash
# Decypher the obfuscated password or salt
rclone reveal PASSWORD
```

## Troubleshooting
### Common Issues
* **"Invalid file format" error**
 - Make sure you're using a file encrypted by rclone
 - Check that the file isn't corrupted
 - Verify the file starts with the rclone magic header
* **"Invalid password" error**
  - Double-check your password
  - Verify the salt is correct (base64 encoded)
  - Make sure you're using the same salt that was used for encryption
* **"File too short" error**
  - The file might be corrupted or incomplete
  - Ensure the entire encrypted file was copied

## Performance Notes
- Large files are processed in chunks for memory efficiency
- The tool can handle files of any size
- Decryption speed depends on your system's CPU performance

## Security Considerations
- The password and salt are handled securely in memory
- No temporary files are created during decryption
- Always verify the integrity of decrypted files
- Keep your rclone configuration secure as it contains encryption keys

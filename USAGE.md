# RClone Decrypt - Usage Examples

This document provides practical examples of how to use rclone-decrypt to decrypt files that were encrypted with rclone.

## Getting Started

### 1. Find Your Encrypted File

Rclone typically creates `.bin` files when encryption is enabled. These files contain the encrypted data with the rclone encryption format.

### 2. Locate Your Encryption Settings

You'll need two pieces of information:
- **Password**: The encryption password you used with rclone
- **Salt**: The salt/password2 used by rclone (usually stored in rclone config)

### 3. Extract the Salt from Rclone Config

```bash
# Show your rclone config with secrets revealed
rclone config show your-crypt-remote --show-secrets

# Look for the 'password2' field - this is your salt (base64 encoded)
```

## Basic Usage Examples

### Example 1: Simple Decryption with Plain Text Salt

```bash
# Basic command structure with plain text salt
./rclone-decrypt \
  --input encrypted_file.bin \
  --output decrypted_file.txt \
  --password "your_encryption_password" \
  --salt "your_plain_text_salt"
```

### Example 2: With Base64 Encoded Salt

```bash
# Using base64 encoded salt from rclone config
./rclone-decrypt \
  --input encrypted_file.bin \
  --output decrypted_file.txt \
  --password "your_encryption_password" \
  --salt "base64_encoded_salt_from_rclone_config"
```

### Example 3: With Verbose Output

```bash
# Add -v flag to see what's happening (plain text salt)
./rclone-decrypt \
  -i encrypted_file.bin \
  -o decrypted_file.txt \
  -p "my_secret_password" \
  -s "my_salt" \
  -v
```

### Example 4: Hex-encoded Salt

```bash
# If your salt is in hex format, prefix with 0x
./rclone-decrypt \
  -i encrypted_file.bin \
  -o decrypted_file.txt \
  -p "my_secret_password" \
  -s "0x48656c6c6f20576f726c64"
```

## Real-world Workflow

### Step 1: Create encrypted files with rclone

```bash
# Set up a crypt remote in rclone
rclone config create mycrypt crypt remote=./storage password=mypass password2=mysalt

# Copy files to encrypted storage
echo "Secret document content" > important.txt
rclone copy important.txt mycrypt:
```

### Step 2: Find the encrypted file

```bash
# List files in storage directory
ls -la storage/
# You'll see files like: vhefql5nsu0kn8v8glqn7fsldk
```

### Step 3: Get your salt

```bash
# Show config with secrets
rclone config show mycrypt --show-secrets
```

Output will look like:
```
[mycrypt]
type = crypt
remote = ./storage
password = *** ENCRYPTED ***
password2 = *** ENCRYPTED ***
```

### Step 4: Decrypt with rclone-decrypt

```bash
# Use the revealed password2 as your salt
./rclone-decrypt \
  -i storage/vhefql5nsu0kn8v8glqn7fsldk \
  -o important_decrypted.txt \
  -p "mypass" \
  -s "your_revealed_salt_here" \
  -v
```

## Troubleshooting

### Common Issues

1. **"Invalid file format" error**
   - Make sure you're using a file encrypted by rclone
   - Check that the file isn't corrupted
   - Verify the file starts with the rclone magic header

2. **"Invalid password" error**
   - Double-check your password
   - Verify the salt is correct (base64 encoded)
   - Make sure you're using the same salt that was used for encryption

3. **"File too short" error**
   - The file might be corrupted or incomplete
   - Ensure the entire encrypted file was copied

### Getting Help

```bash
# Show all available options
./rclone-decrypt --help

# Show version information
./rclone-decrypt --version
```

## Performance Notes

- Large files are processed in chunks for memory efficiency
- The tool can handle files of any size
- Decryption speed depends on your system's CPU performance

## Security Considerations

- The password and salt are handled securely in memory
- No temporary files are created during decryption
- Always verify the integrity of decrypted files
- Keep your rclone configuration secure as it contains encryption keys

#!/bin/bash

# Test script to create a sample encrypted file using rclone and then decrypt it

set -e

# Check if rclone is installed
if ! command -v rclone >/dev/null 2>&1; then
    echo "Error: rclone is not installed. Please install rclone first."
    echo "You can install it with: brew install rclone"
    exit 1
fi

# Create test data
echo "Creating test data..."
echo "This is a test file for rclone encryption/decryption." > test_original.txt
echo "It contains some sample text to verify the decryption works correctly." >> test_original.txt
echo "Line 3 of the test file." >> test_original.txt

# Set up rclone config for encryption
echo "Setting up rclone encryption..."
PASSWORD="example_password_123"
SALT="ZXhhbXBsZXNhbHQ="  # base64 encoded "examplesalt"

# Create a temporary rclone config
mkdir -p ~/.config/rclone
cat > ~/.config/rclone/rclone.conf << EOF
[testcrypt]
type = crypt
remote = ./test_encrypted
password = $PASSWORD
password2 = $SALT
EOF

# Encrypt the file using rclone
echo "Encrypting file with rclone..."
rclone copy test_original.txt testcrypt:

# Find the encrypted file
ENCRYPTED_FILE=$(find test_encrypted -name "*.bin" | head -1)

if [ -z "$ENCRYPTED_FILE" ]; then
    echo "Error: No encrypted .bin file found"
    exit 1
fi

echo "Encrypted file created: $ENCRYPTED_FILE"

# Build our decryption tool
echo "Building rclone-decrypt..."
cargo build --release

# Decrypt the file using our tool
echo "Decrypting file with rclone-decrypt..."
./target/release/rclone-decrypt \
    -i "$ENCRYPTED_FILE" \
    -o test_decrypted.txt \
    -p "$PASSWORD" \
    -s "$SALT" \
    -v

# Verify the decryption
echo "Verifying decryption..."
if cmp -s test_original.txt test_decrypted.txt; then
    echo "✅ SUCCESS: Decryption works correctly!"
    echo "Original and decrypted files are identical."
else
    echo "❌ FAILURE: Decrypted file doesn't match original."
    echo "Original file:"
    cat test_original.txt
    echo -e "\nDecrypted file:"
    cat test_decrypted.txt
    exit 1
fi

# Cleanup
echo "Cleaning up test files..."
rm -f test_original.txt test_decrypted.txt
rm -rf test_encrypted
rm -f ~/.config/rclone/rclone.conf

echo "Test completed successfully!"

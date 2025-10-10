#!/bin/bash
set -e +vx
# test.sh - Test to decrypt an rclone encrypted file

# Check if rclone is installed
! command -v rclone >/dev/null 2>&1 &&
	echo "### 'rclone' not installed, install with:" &&
	echo "    [OSX/brew] brew install rclone" &&
	echo "    [Linux/deb] apt install rclone" &&
	exit 1

echo "--- Creating test data..."
mkdir -p test
echo "This is a test file for rclone encryption/decryption." >test/_original.txt
echo "It contains some sample text to verify the decryption works correctly." >>test/_original.txt
echo "Line 3 of the test file." >>test/_original.txt

echo "--- Setting up rclone encryption..."
conf=~/.config/rclone/rclone.conf
PASSWORD='a_simple_password_123'
PASSWORDX='QCa5bEVOj0BPJVV82ffAFSfwzjA3lAtA6_g-K1WL4ZycRKr72g'
SALT='examplesalt'
SALTX='_n_T-l-foZWmzS4lCpuYlNw3AReZnRcqXrbi'
mkdir -p ~/.config/rclone
	grep -q '^\[testcrypt]$' "$conf" ||
	cat <<-EOF >>"$conf"

		[testcrypt]
		type = crypt
		remote = ./test
		password = $PASSWORDX
		password2 = $SALTX
	EOF

echo "--- Encrypting file with rclone..."
rclone copy test/_original.txt testcrypt:

# Find the encrypted file
encrypted_file=$(echo test/[^_]*)
[[ ! -f $encrypted_file ]] &&
	echo "### No encrypted file found" &&
	exit 2

echo "--- Encrypted file created: $encrypted_file"
echo "Building rclonedecrypt..."
cargo rel

echo "--- Decrypting file with rclonedecrypt..."
./rclonedecrypt "$encrypted_file" -o test/_decrypted.txt -p $PASSWORD -s $SALT -v

echo "--- Verifying decryption..."
if diff test/_original.txt test/_decrypted.txt >/dev/null
then # Same
	echo "--- ✅ SUCCESS: Decryption works correctly!"
	echo "=== Original and decrypted files are identical"
else # Different
	echo "--- ❌ FAILURE: Decrypted file doesn't match original..."
	echo "--- Original file:"
	cat test/_original.txt
	echo -e "\n--- Decrypted file:"
	cat -A test/_decrypted.txt
	echo "--- See directory 'test'"
	exit 3
fi

echo "--- Cleaning up test files..."
rm -rf test
#rm -f ~/.config/rclone/rclone.conf

echo "=== Test completed successfully"

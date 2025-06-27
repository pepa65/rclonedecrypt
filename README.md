# RClone Decrypt

A cross-platform command-line tool to decrypt `.bin` files created with rclone encryption.

## Features

- **Cross-platform**: Compiles to native binaries for Windows and macOS
- **Fast decryption**: Efficient handling of large encrypted files
- **rclone compatible**: Implements the same encryption algorithm used by rclone
- **Command-line interface**: Easy to use from terminal or scripts

## Installation

### Prerequisites
- Rust toolchain (install from [rustup.rs](https://rustup.rs/))

### Building from source

```bash
# Clone the repository
git clone <your-repo-url>
cd rclone-decrypt

# Build the project
cargo build --release

# The binary will be available at target/release/rclone-decrypt
```

### Quick build with make

```bash
# Build release version
make release

# Run tests
make test

# Cross-compile for multiple platforms
make cross-compile

# Show all available targets
make help
```

### Cross-compilation

To build for different platforms:

```bash
# Use the provided build script
./build.sh

# Or manually with cargo
cargo build --release --target x86_64-apple-darwin    # Intel macOS
cargo build --release --target aarch64-apple-darwin   # Apple Silicon macOS
cargo build --release --target x86_64-pc-windows-gnu  # Windows (requires mingw-w64)
```

### Download pre-built binaries

Check the [Releases](https://github.com/your-username/rclone-decrypt/releases) page for pre-built binaries for:
- macOS (Intel x86_64)
- macOS (Apple Silicon ARM64) 
- Windows (x86_64)
- Linux (x86_64)

## Usage

```bash
rclone-decrypt -i encrypted_file.bin -o decrypted_file.txt -p "your_password" -s "your_salt"
```

### Command-line Options

- `-i, --input <FILE>`: Input encrypted .bin file (required)
- `-o, --output <FILE>`: Output decrypted file (required)
- `-p, --password <PASSWORD>`: Encryption password (required)
- `-s, --salt <SALT>`: Salt used for encryption, accepts multiple formats:
  - Plain text: `"mysalt"` 
  - Base64 encoded: `"dGVzdHNhbHQ="`
  - Hex with 0x prefix: `"0x7465737473616c74"`
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Show help message

### Examples

```bash
# Decrypt with plain text salt
./rclone-decrypt -i secret.bin -o secret.txt -p "mypassword" -s "mysalt"

# Decrypt with base64 encoded salt
./rclone-decrypt -i secret.bin -o secret.txt -p "mypassword" -s "dGVzdHNhbHQ="

# Decrypt with hex encoded salt
./rclone-decrypt -i secret.bin -o secret.txt -p "mypassword" -s "0x7465737473616c74"

# Verbose output
./rclone-decrypt -i secret.bin -o secret.txt -p "mypassword" -s "mysalt" -v
```

### Getting the salt from rclone config

If you encrypted files with rclone, you can find the salt in your rclone configuration:

1. **From rclone config file** (`~/.config/rclone/rclone.conf`):
   ```
   [mycrypt]
   type = crypt
   remote = /path/to/storage
   password = <obscured_password>
   password2 = <obscured_salt>
   ```

2. **Decrypt the obscured salt**:
   ```bash
   # Use rclone to reveal the obscured password2 (salt)
   rclone config show mycrypt --show-secrets
   ```
   
3. **Use the revealed salt** in rclone-decrypt (it will be base64 encoded)

## How it works

This tool implements the rclone encryption format:

1. **File Format**: Files start with the magic header "RCLONE\x00\x00"
2. **Key Derivation**: Uses scrypt with N=16384, r=8, p=1 to derive a 256-bit key
3. **Encryption**: AES-256 in CTR mode
4. **IV**: 16-byte initialization vector stored after the header

## Technical Details

- **Algorithm**: AES-256-CTR
- **Key derivation**: scrypt (N=16384, r=8, p=1)
- **Key size**: 256 bits (32 bytes)
- **IV size**: 128 bits (16 bytes)
- **Salt size**: 64 bits (8 bytes)

## Error Handling

The tool provides clear error messages for common issues:
- Invalid file format
- Incorrect password
- File too short
- Invalid salt format

## Development

### Running tests

```bash
cargo test
```

### Code structure

- `src/main.rs`: Main application entry point
- `src/cli.rs`: Command-line interface definition
- `src/decrypt.rs`: Core decryption logic
- `src/error.rs`: Error types and handling

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## Security Notes

- Passwords are handled in memory and should be cleared after use
- This tool is designed for legitimate decryption of your own files
- Always verify the integrity of decrypted files

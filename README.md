[![version](https://img.shields.io/crates/v/rclonedecrypt.svg)](https://crates.io/crates/rclonedecrypt)
[![build](https://github.com/pepa65/rclonedecrypt/actions/workflows/ci.yml/badge.svg)](https://github.com/pepa65/rclonedecrypt/actions/workflows/ci.yml)
[![dependencies](https://deps.rs/repo/github/pepa65/rclonedecrypt/status.svg)](https://deps.rs/repo/github/pepa65/rclonedecrypt)
[![docs](https://img.shields.io/badge/docs-rclonedecrypt-blue.svg)](https://docs.rs/crate/rclonedecrypt/latest)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/pepa65/rclonedecrypt/blob/master/LICENSE)
[![downloads](https://img.shields.io/crates/d/rclonedecrypt.svg)](https://crates.io/crates/rclonedecrypt)

# rclonedecrypt 0.2.34
**CLI to decrypt rclone-encrypted files**

## Features
* **Cross-platform**: Compiles to native binaries for Linux, Windows and macOS
* **Fast decryption**: Efficient handling of large encrypted files
* **rclone compatible**: Implements the same encryption algorithm used by rclone
* **Command-line interface**: Easy to use from terminal or scripts
* **Repo**: https://github.com/pepa65/rclonedecrypt
* **After**: https://github.com/mcolatosti/rclonedecrypt
  - Removed various password options
  - Removed the -i/--input flag (made it a positional argument)
  - Removed inline-tests
  - Adjusted documentation for rclone 1.59

## Installation
### Prerequisites
- Rust toolchain (install from [rustup.rs](https://rustup.rs/))

### Building from source
```bash
git clone https://github.com/pepa65/rclonedecrypt
cd rclonedecrypt
cargo rel  # The binary gets symlinked to `rclonedecrypt`
```

### Run test
`./test.sh`

### Build different targets
* Linux: `cargo rel` (gives standalone static binary)
* Intel macOS: `cargo build --release --target x86_64-apple-darwin`
* Apple Silicon macOS: `cargo build --release --target aarch64-apple-darwin`
* Windows (with `mingw-w64`): `cargo build --release --target x86_64-pc-windows-gnu`

### Download pre-built binaries
Check the [Releases](https://github.com/pepa65/rclonedecrypt/releases) page

## Usage
### Help
```
rclonedecrypt 0.2.34 - CLI to decrypt rclone-encrypted files
USAGE: rclonedecrypt [OPTIONS] --output <FILE> --password <PASSWORD> --salt <SALT> <FILE>
OPTIONS:
  -o, --output <FILE>        Output decrypted file
  -p, --password <PASSWORD>  Encryption password
  -s, --salt <SALT>          Salt used for encryption
  -v, --verbose              Enable verbose output
  -h, --help                 Print help
  -V, --version              Print version
```

### Example
`rclonedecrypt encrypted_file.jpg.bin -o decrypted_file.jpg -p 'PASSWORD' -s 'SALT'

## Technical Details
* **Algorithm**: AES-256-CTR
* **Key derivation**: scrypt (N=16384, r=8, p=1)
* **Key size**: 256 bits (32 bytes)
* **IV size**: 128 bits (16 bytes)
* **Salt size**: 64 bits (8 bytes)

### Code structure
* `src/main.rs`: Main application entry point
* `src/cli.rs`: Command-line interface definition
* `src/decrypt.rs`: Core decryption logic
* `src/error.rs`: Error types and handling

## License
This project is licensed under the MIT License - see the LICENSE file for details.

## Security Notes
- Passwords are handled in memory and should be cleared after use.
- This tool is intended for legitimate decryption of files you have the right to.
- Always verify the integrity of decrypted files.

## Other documents
* DECRYPTION_SUCCESS.md
* USAGE.md

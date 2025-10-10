mod cli;
mod decrypt;
mod error;

use crate::cli::build_cli;
use crate::decrypt::RcloneDecryptor;
use crate::error::DecryptionResult;
use std::process;

fn main() {
	if let Err(e) = run() {
		eprintln!("Error: {}", e);
		process::exit(1);
	}
}

fn run() -> DecryptionResult<()> {
	let matches = build_cli().get_matches();
	let input_file = matches.get_one::<String>("input").unwrap();
	let output_file = matches.get_one::<String>("output").unwrap();
	let password = matches.get_one::<String>("password").unwrap();
	let salt = matches.get_one::<String>("salt").unwrap();
	let verbose = matches.get_flag("verbose");
	if verbose {
		println!("Input file: {}", input_file);
		println!("Output file: {}", output_file);
		println!("Password: (provided)");
		println!("Salt: {}", &salt);
	}
	let decryptor = RcloneDecryptor::new(password.to_string(), salt.to_string())?;
	if verbose {
		println!("Debug: Using salt: {} bytes", decryptor.get_salt_debug().len());
	}
	println!("Decrypting using NaCl SecretBox (XSalsa20 + Poly1305)...");
	decryptor.decrypt_file(input_file, output_file)?;
	println!("Decryption completed successfully!");
	Ok(())
}

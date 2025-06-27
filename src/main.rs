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
    let password = matches.get_one::<String>("password").map(|s| s.clone()).unwrap_or_default();
    let salt = matches.get_one::<String>("salt").map(|s| s.clone()).unwrap_or_default();
    let verbose = matches.get_flag("verbose");
    let auto_detect = matches.get_flag("auto");
    let interactive = matches.get_flag("interactive");
    let wordlist = matches.get_one::<String>("wordlist");

    if verbose {
        println!("Input file: {}", input_file);
        println!("Output file: {}", output_file);
        println!("Password: {}", if password.is_empty() { "(not provided)" } else { "(provided)" });
        println!("Salt: {}", if salt.is_empty() { "(empty)" } else { &salt });
        println!("Auto-detect: {}", auto_detect);
        println!("Interactive: {}", interactive);
        if let Some(wl) = wordlist {
            println!("Wordlist: {}", wl);
        }
    }

    // Interactive mode
    if interactive {
        println!("Starting interactive mode...");
        return RcloneDecryptor::interactive_decrypt(input_file, output_file);
    }

    // Wordlist mode
    if let Some(wordlist_file) = wordlist {
        println!("Loading passwords from wordlist: {}", wordlist_file);
        let wordlist_content = std::fs::read_to_string(wordlist_file)?;
        let passwords: Vec<String> = wordlist_content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        
        println!("Loaded {} passwords from wordlist", passwords.len());
        
        if let Ok(found_password) = RcloneDecryptor::try_multiple_passwords(input_file, output_file, &passwords) {
            println!("Success! The file was decrypted using password: '{}'", found_password);
            return Ok(());
        } else {
            println!("None of the {} passwords in the wordlist worked.", passwords.len());
            return Err(crate::error::DecryptionError::InvalidPassword);
        }
    }

    // Standard mode (requires password)
    if password.is_empty() {
        eprintln!("Error: Password is required unless using --interactive or --wordlist mode");
        eprintln!("Use --help for more information");
        std::process::exit(1);
    }

    let decryptor = RcloneDecryptor::new(password, salt)?;
    
    if verbose {
        println!("Debug: Using salt: {} bytes", decryptor.get_salt_debug().len());
    }

    println!("Decrypting using NaCl SecretBox (XSalsa20 + Poly1305)...");
    
    if auto_detect {
        println!("Auto-detecting rclone configuration...");
        decryptor.auto_decrypt(input_file, output_file)?;
    } else {
        decryptor.decrypt_file(input_file, output_file)?;
    }
    
    println!("Decryption completed successfully!");

    Ok(())
}

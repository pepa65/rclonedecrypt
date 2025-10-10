use clap::{Arg, Command, command};

pub fn build_cli() -> Command {
	command!()
		.help_template("{name} {version} - {about}\nUSAGE: {usage}\nOPTIONS:\n{options}")
		.arg(Arg::new("input").value_name("FILE").help("Input rclone-encrypted file").required(true))
		.arg(Arg::new("output").short('o').long("output").value_name("FILE").help("Output decrypted file").required(true))
		.arg(Arg::new("password").short('p').long("password").value_name("PASSWORD").help("Encryption password").required(true))
		.arg(Arg::new("salt").short('s').long("salt").value_name("SALT").help("Salt used for encryption").required(true))
		.arg(Arg::new("verbose").short('v').long("verbose").help("Enable verbose output").action(clap::ArgAction::SetTrue))
}

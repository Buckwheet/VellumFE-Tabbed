/// Standalone eAccess authentication test tool.
/// Usage: eaccess-test <account> <password> [game_code] [character]
/// Example: eaccess-test hoggz mypassword GS4 Brashka
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: eaccess-test <account> <password> [game_code] [character]");
        eprintln!("  game_code defaults to GS4");
        eprintln!("  character defaults to first on account");
        std::process::exit(1);
    }

    let account = &args[1];
    let password = &args[2];
    let game_code = args.get(3).map(|s| s.as_str()).unwrap_or("GS4");
    let character_filter = args.get(4).map(|s| s.as_str());

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("vellum-fe");

    println!("=== eAccess Raw Debug Test ===");
    println!("Account:   {}", account);
    println!("Game:      {}", game_code);
    println!();

    match vellum_fe_tabbed::network::eaccess_raw_debug(
        account,
        password,
        game_code,
        character_filter.unwrap_or("Brashka"),
        &data_dir,
    ) {
        Ok(()) => println!("\nSUCCESS"),
        Err(e) => {
            eprintln!("\nFAIL: {:#}", e);
            std::process::exit(1);
        }
    }
}

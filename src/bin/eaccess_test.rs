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

    println!("=== eAccess Test ===");
    println!("Account:   {}", account);
    println!("Game:      {}", game_code);
    println!("Data dir:  {}", data_dir.display());
    println!();

    // Step 1: fetch characters
    println!("[Step 1] Fetching character list...");
    let chars = match vellum_fe_tabbed::network::fetch_characters_for_account(
        account, password, game_code, &data_dir,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FAIL (fetch_characters): {:#}", e);
            std::process::exit(1);
        }
    };
    println!("  Characters: {:?}", chars);

    // Step 2: pick character
    let character = match character_filter {
        Some(name) => {
            if !chars.iter().any(|c| c.eq_ignore_ascii_case(name)) {
                eprintln!("FAIL: character '{}' not found in {:?}", name, chars);
                std::process::exit(1);
            }
            name.to_string()
        }
        None => match chars.first() {
            Some(c) => c.clone(),
            None => {
                eprintln!("FAIL: no characters on account");
                std::process::exit(1);
            }
        },
    };
    println!("  Selected:   {}", character);
    println!();

    // Step 3: authenticate and launch
    println!("[Step 2] Authenticating and launching {}...", character);
    match vellum_fe_tabbed::network::authenticate_and_launch(
        account, password, &character, game_code, &data_dir,
    ) {
        Ok((host, port, key)) => {
            println!("  SUCCESS!");
            println!("  Game host: {}", host);
            println!("  Game port: {}", port);
            println!("  Key:       {}...", &key[..key.len().min(8)]);
        }
        Err(e) => {
            eprintln!("  FAIL: {:#}", e);
            std::process::exit(1);
        }
    }
}

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use a2s::A2SClient;
use a2s::HEADER_CHALLENGE;
use a2s::info::INFO_REQUEST;
use a2s::info::Info;
use a2s::players::PLAYER_REQUEST;
use a2s::rules::RULES_REQUEST;

fn slug(addr: &str) -> String {
    addr.replace([':', '.', '[', ']'], "_")
}

/// Resolve the query address from a game address and optional query port offset.
/// If `query_port_offset` is non-zero, the query port is `game_port + offset`.
fn resolve_query_addr(game_addr: &str, query_port_offset: i32) -> String {
    if query_port_offset == 0 {
        return game_addr.to_string();
    }

    // Parse the address to extract host and port
    if let Some((host, port_str)) = game_addr.rsplit_once(':')
        && let Ok(game_port) = port_str.parse::<u16>()
    {
        let query_port = (game_port as i32 + query_port_offset) as u16;
        return format!("{host}:{query_port}");
    }

    game_addr.to_string()
}

fn capture_info(client: &A2SClient, addr: &str) -> Option<Vec<u8>> {
    match client.send(&INFO_REQUEST, addr) {
        Ok(data) => {
            if !data.is_empty() && data[0] == HEADER_CHALLENGE && data.len() >= 5 {
                let challenge = i32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                let mut query = Vec::with_capacity(29);
                query.extend_from_slice(&INFO_REQUEST);
                query.extend_from_slice(&challenge.to_le_bytes());
                match client.send(&query, addr) {
                    Ok(data) => Some(data),
                    Err(e) => {
                        eprintln!("  info (challenged) failed: {e}");
                        None
                    }
                }
            } else {
                Some(data)
            }
        }
        Err(e) => {
            eprintln!("  info failed: {e}");
            None
        }
    }
}

fn capture(client: &A2SClient, game_addr: &str, query_addr: &str, output_dir: &Path) {
    let s = slug(game_addr);

    let info_data = capture_info(client, query_addr);

    // Parse info to get app_id for directory organization
    let app_id = info_data
        .as_deref()
        .and_then(|data| Info::from_reader(data).ok().map(|info| info.app_id));

    let dir = match app_id {
        Some(id) => output_dir.join(id.0.to_string()),
        None => output_dir.join("unknown"),
    };
    fs::create_dir_all(&dir).expect("failed to create app_id directory");

    if let Some(data) = &info_data {
        write_fixture(&dir, &s, "info", data);
    }

    match client.do_challenge_request(query_addr, &PLAYER_REQUEST) {
        Ok(data) => write_fixture(&dir, &s, "players", &data),
        Err(e) => eprintln!("  players failed: {e}"),
    }

    match client.do_challenge_request(query_addr, &RULES_REQUEST) {
        Ok(data) => write_fixture(&dir, &s, "rules", &data),
        Err(e) => eprintln!("  rules failed: {e}"),
    }
}

fn write_fixture(dir: &Path, slug: &str, kind: &str, data: &[u8]) {
    let path = dir.join(format!("{slug}_{kind}.bin"));
    let mut f = fs::File::create(&path).expect("failed to create fixture file");
    f.write_all(data).expect("failed to write fixture");
    println!("  wrote {} ({} bytes)", path.display(), data.len());
}

fn query(client: &A2SClient, addr: &str) {
    match client.info(addr) {
        Ok(info) => {
            println!("  name:        {}", info.name);
            println!("  map:         {}", info.map);
            println!("  game:        {}", info.game);
            println!("  players:     {}/{}", info.players, info.max_players);
            println!("  bots:        {}", info.bots);
            println!("  server_type: {:?}", info.server_type);
            println!("  server_os:   {:?}", info.server_os);
            println!("  vac:         {}", info.vac);
            println!("  version:     {}", info.version);
            if let Some(port) = info.extended_server_info.port {
                println!("  game_port:   {port}");
            }
            if let Some(keywords) = &info.extended_server_info.keywords {
                println!("  keywords:    {keywords}");
            }
            if let Some(game_id) = info.extended_server_info.game_id {
                println!("  game_id:     {}", game_id.0);
            }
        }
        Err(e) => eprintln!("  info failed: {e}"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut query_port_offset: i32 = 0;
    let mut positional = Vec::new();
    let mut iter = args[1..].iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--query-port-offset" => {
                let val = iter.next().unwrap_or_else(|| {
                    eprintln!("--query-port-offset requires a value");
                    std::process::exit(1);
                });
                query_port_offset = val.parse().unwrap_or_else(|e| {
                    eprintln!("invalid --query-port-offset value: {e}");
                    std::process::exit(1);
                });
            }
            _ => positional.push(arg.as_str()),
        }
    }

    if positional.is_empty() {
        eprintln!(
            "Usage: {} [--query-port-offset N] <command> [args...]",
            args[0]
        );
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  query <addr> [<addr>...]          Query servers and print info");
        eprintln!("  capture <output_dir> <addr> [...]  Capture raw responses to files");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --query-port-offset N  Add N to each game port to get the query port");
        std::process::exit(1);
    }

    let command = positional[0];
    let rest = &positional[1..];

    let client = A2SClient::new(Duration::new(5, 0)).expect("failed to create A2S client");

    match command {
        "query" => {
            if rest.is_empty() {
                eprintln!("Usage: query <addr> [<addr>...]");
                std::process::exit(1);
            }
            for game_addr in rest {
                let query_addr = resolve_query_addr(game_addr, query_port_offset);
                println!("{game_addr}:");
                query(&client, &query_addr);
            }
        }
        "capture" => {
            if rest.len() < 2 {
                eprintln!("Usage: capture <output_dir> <addr> [<addr>...]");
                std::process::exit(1);
            }
            let output_dir = Path::new(rest[0]);
            fs::create_dir_all(output_dir).expect("failed to create output directory");

            for game_addr in &rest[1..] {
                let query_addr = resolve_query_addr(game_addr, query_port_offset);
                if query_port_offset != 0 {
                    println!("Querying {game_addr} (query port: {query_addr})...");
                } else {
                    println!("Querying {game_addr}...");
                }
                capture(&client, game_addr, &query_addr, output_dir);
            }
        }
        other => {
            eprintln!("Unknown command: {other}");
            std::process::exit(1);
        }
    }
}

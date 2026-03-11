use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use a2s::A2SClient;
use a2s::info::INFO_REQUEST;
use a2s::info::Info;
use a2s::players::PLAYER_REQUEST;
use a2s::rules::RULES_REQUEST;

fn slug(addr: &str) -> String {
    addr.replace([':', '.', '[', ']'], "_")
}

fn capture_info(client: &A2SClient, addr: &str) -> Option<Vec<u8>> {
    match client.send(&INFO_REQUEST, addr) {
        Ok(data) => {
            if !data.is_empty() && data[0] == b'A' && data.len() >= 5 {
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

fn capture(client: &A2SClient, addr: &str, output_dir: &Path) {
    let s = slug(addr);

    let info_data = capture_info(client, addr);

    // Parse info to get app_id for directory organization
    let app_id = info_data
        .as_deref()
        .and_then(|data| Info::from_reader(data).ok().map(|info| info.app_id));

    let dir = match app_id {
        Some(id) => output_dir.join(id.to_string()),
        None => output_dir.join("unknown"),
    };
    fs::create_dir_all(&dir).expect("failed to create app_id directory");

    if let Some(data) = &info_data {
        write_fixture(&dir, &s, "info", data);
    }

    match client.do_challenge_request(addr, &PLAYER_REQUEST) {
        Ok(data) => write_fixture(&dir, &s, "players", &data),
        Err(e) => eprintln!("  players failed: {e}"),
    }

    match client.do_challenge_request(addr, &RULES_REQUEST) {
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <output_dir> <addr> [<addr>...]", args[0]);
        std::process::exit(1);
    }

    let output_dir = Path::new(&args[1]);
    fs::create_dir_all(output_dir).expect("failed to create output directory");

    let client = A2SClient::new().expect("failed to create A2S client");

    for addr in &args[2..] {
        println!("Querying {addr}...");
        capture(&client, addr, output_dir);
    }
}

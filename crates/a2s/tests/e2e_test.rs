use std::fs;
use std::net::UdpSocket;
use std::time::Duration;

use a2s::A2SClient;

const SINGLE_PACKET_HEADER: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

fn fixture(path: &str) -> Vec<u8> {
    let full = format!("tests/fixtures/{path}");
    fs::read(&full).unwrap_or_else(|e| panic!("failed to read fixture {full}: {e}"))
}

/// Derive a snapshot name from a fixture path:
/// `320/74_91_118_209_27015_info.bin` -> `320__74_91_118_209_27015_info`
fn snap_name(prefix: &str, fixture_path: &str) -> String {
    let base = fixture_path
        .strip_suffix(".bin")
        .unwrap_or(fixture_path)
        .replace('/', "__");
    format!("{prefix}__{base}")
}

/// Wrap a fixture payload with the single-packet response header.
fn single_packet(payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.extend_from_slice(&SINGLE_PACKET_HEADER);
    buf.extend_from_slice(payload);
    buf
}

/// Build a challenge response: single-packet header + 'A' + challenge_value (i32 LE).
fn challenge_response(challenge: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(9);
    buf.extend_from_slice(&SINGLE_PACKET_HEADER);
    buf.push(b'A');
    buf.extend_from_slice(&challenge.to_le_bytes());
    buf
}

/// Spawn a mock A2S server that sends each response in `responses` for each
/// packet it receives, in order. Returns the socket address to connect to.
fn mock_server(responses: Vec<Vec<u8>>) -> std::net::SocketAddr {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        for response in responses {
            let (_, src) = socket.recv_from(&mut buf).unwrap();
            socket.send_to(&response, src).unwrap();
        }
    });

    addr
}

// -- Sync client tests --

#[test]
fn sync_info() {
    let path = "320/74_91_118_209_27015_info.bin";
    let payload = fixture(path);
    let addr = mock_server(vec![single_packet(&payload)]);

    let client = A2SClient::new(Duration::from_secs(5)).unwrap();
    let info = client.info(addr).unwrap();

    insta::assert_debug_snapshot!(snap_name("sync", path), info);
}

#[test]
fn sync_info_with_challenge() {
    let path = "0/51_38_89_140_2302_info.bin";
    let payload = fixture(path);
    let addr = mock_server(vec![
        challenge_response(0x12345678),
        single_packet(&payload),
    ]);

    let client = A2SClient::new(Duration::from_secs(5)).unwrap();
    let info = client.info(addr).unwrap();

    insta::assert_debug_snapshot!(snap_name("sync", path), info);
}

#[test]
fn sync_players() {
    let path = "320/74_91_118_209_27015_players.bin";
    let payload = fixture(path);
    let addr = mock_server(vec![
        challenge_response(0xAABBCCDD_u32 as i32),
        single_packet(&payload),
    ]);

    let client = A2SClient::new(Duration::from_secs(5)).unwrap();
    let players = client.players(addr).unwrap();

    insta::assert_debug_snapshot!(snap_name("sync", path), players);
}

#[test]
fn sync_rules() {
    let path = "320/74_91_118_209_27015_rules.bin";
    let payload = fixture(path);
    let addr = mock_server(vec![
        challenge_response(0x11223344),
        single_packet(&payload),
    ]);

    let client = A2SClient::new(Duration::from_secs(5)).unwrap();
    let rules = client.rules(addr).unwrap();

    insta::assert_debug_snapshot!(snap_name("sync", path), rules);
}

// -- Async client tests --

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use tokio::net::UdpSocket as TokioUdpSocket;

    /// Async mock server using tokio.
    async fn mock_server_async(responses: Vec<Vec<u8>>) -> std::net::SocketAddr {
        let socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            for response in responses {
                let (_, src) = socket.recv_from(&mut buf).await.unwrap();
                socket.send_to(&response, src).await.unwrap();
            }
        });

        addr
    }

    #[tokio::test]
    async fn async_info() {
        let path = "320/74_91_118_209_27015_info.bin";
        let payload = fixture(path);
        let addr = mock_server_async(vec![single_packet(&payload)]).await;

        let client = a2s::nonblocking::A2SClient::new().await.unwrap();
        let info = client.info(addr).await.unwrap();

        insta::assert_debug_snapshot!(snap_name("async", path), info);
    }

    #[tokio::test]
    async fn async_info_with_challenge() {
        let path = "0/51_38_89_140_2302_info.bin";
        let payload = fixture(path);
        let addr = mock_server_async(vec![
            challenge_response(0x12345678),
            single_packet(&payload),
        ])
        .await;

        let client = a2s::nonblocking::A2SClient::new().await.unwrap();
        let info = client.info(addr).await.unwrap();

        insta::assert_debug_snapshot!(snap_name("async", path), info);
    }

    #[tokio::test]
    async fn async_players() {
        let path = "320/74_91_118_209_27015_players.bin";
        let payload = fixture(path);
        let addr = mock_server_async(vec![
            challenge_response(0xAABBCCDD_u32 as i32),
            single_packet(&payload),
        ])
        .await;

        let client = a2s::nonblocking::A2SClient::new().await.unwrap();
        let players = client.players(addr).await.unwrap();

        insta::assert_debug_snapshot!(snap_name("async", path), players);
    }

    #[tokio::test]
    async fn async_rules() {
        let path = "320/74_91_118_209_27015_rules.bin";
        let payload = fixture(path);
        let addr = mock_server_async(vec![
            challenge_response(0x11223344),
            single_packet(&payload),
        ])
        .await;

        let client = a2s::nonblocking::A2SClient::new().await.unwrap();
        let rules = client.rules(addr).await.unwrap();

        insta::assert_debug_snapshot!(snap_name("async", path), rules);
    }
}

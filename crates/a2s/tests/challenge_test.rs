use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use a2s::A2SClient;

/// Wrap a payload in the single-packet header (0xFFFFFFFF).
fn single_packet(payload: &[u8]) -> Vec<u8> {
    let mut pkt = vec![0xFF, 0xFF, 0xFF, 0xFF];
    pkt.extend_from_slice(payload);
    pkt
}

/// Build a challenge response: header 'A' + challenge number (little-endian i32).
fn challenge_response(challenge: i32) -> Vec<u8> {
    let mut payload = vec![b'A'];
    payload.extend_from_slice(&challenge.to_le_bytes());
    single_packet(&payload)
}

/// A minimal A2S_PLAYER response with 0 players.
fn empty_player_response() -> Vec<u8> {
    // header 'D', player_count 0
    single_packet(&[b'D', 0x00])
}

#[test]
fn challenge_retry_on_double_challenge() {
    let server = UdpSocket::bind("127.0.0.1:0").unwrap();
    let server_addr = server.local_addr().unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    let handle = thread::spawn(move || {
        let mut buf = [0u8; 1400];

        // 1) Receive initial request (challenge = -1), reply with challenge 1000
        let (n, client_addr) = server.recv_from(&mut buf).unwrap();
        assert!(n >= 9, "initial request too short");
        server
            .send_to(&challenge_response(1000), client_addr)
            .unwrap();

        // 2) Receive retry with challenge 1000, reply with a SECOND challenge 2000
        let (n, client_addr) = server.recv_from(&mut buf).unwrap();
        assert!(n >= 9, "first retry too short");
        let challenge = i32::from_le_bytes(buf[5..9].try_into().unwrap());
        assert_eq!(challenge, 1000, "client should echo back challenge 1000");
        server
            .send_to(&challenge_response(2000), client_addr)
            .unwrap();

        // 3) Receive retry with challenge 2000, reply with actual player data
        let (n, client_addr) = server.recv_from(&mut buf).unwrap();
        assert!(n >= 9, "second retry too short");
        let challenge = i32::from_le_bytes(buf[5..9].try_into().unwrap());
        assert_eq!(challenge, 2000, "client should echo back challenge 2000");
        server
            .send_to(&empty_player_response(), client_addr)
            .unwrap();
    });

    let client = A2SClient::new(Duration::from_secs(5)).unwrap();
    let players = client.players(server_addr).unwrap();
    assert!(players.is_empty());

    handle.join().unwrap();
}

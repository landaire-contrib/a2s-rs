#![no_main]

use a2s::DeOptions;
use a2s::players::Player;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Wrapper that pairs a DeOptions with players for serialization.
/// Player has no to_bytes(), so we serialize manually.
#[derive(Arbitrary, Debug)]
struct FuzzPlayers {
    options: DeOptions,
    players: Vec<Player>,
}

fn write_cstring(buf: &mut Vec<u8>, s: &[u8]) {
    for &b in s {
        if b != 0 {
            buf.push(b);
        }
    }
    buf.push(0);
}

impl FuzzPlayers {
    fn to_bytes(&self) -> Vec<u8> {
        let count = self.players.len().min(255) as u8;
        let mut buf = Vec::with_capacity(64);
        buf.push(0x44); // header
        buf.push(count);

        for p in self.players.iter().take(count as usize) {
            buf.push(p.index);
            write_cstring(&mut buf, &p.name);
            buf.extend_from_slice(&p.score.to_le_bytes());
            buf.extend_from_slice(&p.duration.to_le_bytes());
            if let Some(ship) = &p.the_ship {
                buf.extend_from_slice(&ship.deaths.to_le_bytes());
                buf.extend_from_slice(&ship.money.to_le_bytes());
            }
        }

        buf
    }
}

fuzz_target!(|input: FuzzPlayers| {
    let bytes = input.to_bytes();
    let _ = Player::from_reader(bytes.as_slice(), &input.options);
});

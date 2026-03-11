//! Parser for the Arma 3 Server Browser Protocol 2 - v3.
//!
//! Arma 3 (and DayZ) servers encode mod lists, signatures, and server metadata
//! into the A2S_RULES response using a custom binary format. The binary data is
//! escaped, split across multiple rules as 124-byte chunks, and reassembled by
//! the client.
//!
//! Reference: <https://community.bistudio.com/wiki/Arma_3:_ServerBrowserProtocol3>

use std::io::Cursor;
use std::io::Read;

use bstr::BString;
use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Serialize;

use super::Rule;
use crate::errors::Error;
use crate::errors::Result;

/// A Steam Workshop file ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct WorkshopId(pub u64);

/// A 4-byte content hash for a mod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ModHash(pub u32);

/// A mod entry from the server's mod list.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[non_exhaustive]
pub struct Mod {
    pub hash: ModHash,
    pub is_dlc: bool,
    pub steam_id: WorkshopId,
    pub name: BString,
}

/// Parsed Arma 3 / DayZ server browser protocol data.
///
/// This is extracted from the binary chunks embedded in A2S_RULES responses.
/// Standard key-value rules (e.g. "island", "platform") are returned separately.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[non_exhaustive]
pub struct Arma3Rules {
    /// Protocol version (typically 2).
    pub version: u32,

    /// Server mods.
    pub mods: Vec<Mod>,

    /// Signature/author names required by the server.
    pub signatures: Vec<BString>,

    /// Server description or trailing text, if present.
    pub description: BString,

    /// Standard key-value rules that are not part of the binary protocol
    /// (e.g. "island", "platform", "dedicated").
    pub rules: Vec<Rule>,
}

impl Arma3Rules {
    /// Parse Arma 3 server browser protocol data from a list of A2S_RULES.
    ///
    /// Rules with 2-byte keys where both bytes are < 0x20 are treated as binary
    /// chunks. All other rules are returned as standard key-value pairs.
    pub fn from_rules(rules: &[Rule]) -> Result<Self> {
        let mut chunks: Vec<(u8, &[u8])> = Vec::new();
        let mut standard: Vec<Rule> = Vec::new();

        for rule in rules {
            if rule.name.len() == 2 && rule.name[0] < 0x20 && rule.name[1] < 0x20 {
                chunks.push((rule.name[0], &rule.value));
            } else {
                standard.push(rule.clone());
            }
        }

        if chunks.is_empty() {
            return Err(Error::NoBinaryChunks);
        }

        chunks.sort_by_key(|&(idx, _)| idx);

        // Concatenate chunk payloads
        let total_len: usize = chunks.iter().map(|(_, v)| v.len()).sum();
        let mut raw = Vec::with_capacity(total_len);
        for (_, payload) in &chunks {
            raw.extend_from_slice(payload);
        }

        // Unescape: 0x01 0x01 → 0x01, 0x01 0x02 → 0x00, 0x01 0x03 → 0xFF
        let stream = unescape(&raw);
        let mut cursor = Cursor::new(&stream);

        let version = cursor.read_u32::<LittleEndian>()?;
        let mod_count = cursor.read_u8()?;

        let mut mods = Vec::with_capacity(mod_count as usize);
        for _ in 0..mod_count {
            let hash = ModHash(cursor.read_u32::<LittleEndian>()?);

            let flags_byte = cursor.read_u8()?;
            let is_dlc = flags_byte & 0x10 != 0;
            let steam_id_len = (flags_byte & 0x0F) as usize;

            let steam_id = read_var_uint(&mut cursor, steam_id_len)?;

            let name_len = cursor.read_u8()? as usize;
            let mut name_buf = vec![0u8; name_len];
            cursor.read_exact(&mut name_buf)?;

            mods.push(Mod {
                hash,
                is_dlc,
                steam_id: WorkshopId(steam_id),
                name: BString::new(name_buf),
            });
        }

        let sig_count = cursor.read_u8()?;
        let mut signatures = Vec::with_capacity(sig_count as usize);
        for _ in 0..sig_count {
            let sig_len = cursor.read_u8()? as usize;
            let mut sig_buf = vec![0u8; sig_len];
            cursor.read_exact(&mut sig_buf)?;
            signatures.push(BString::new(sig_buf));
        }

        // Remaining bytes are the server description
        let pos = cursor.position() as usize;
        let description = BString::new(stream[pos..].to_vec());

        Ok(Arma3Rules {
            version,
            mods,
            signatures,
            description,
            rules: standard,
        })
    }
}

/// Unescape the Arma 3 binary protocol encoding.
///
/// Escape sequences:
/// - `0x01 0x01` → `0x01`
/// - `0x01 0x02` → `0x00`
/// - `0x01 0x03` → `0xFF`
fn unescape(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0x01 && i + 1 < data.len() {
            match data[i + 1] {
                0x01 => {
                    out.push(0x01);
                    i += 2;
                    continue;
                }
                0x02 => {
                    out.push(0x00);
                    i += 2;
                    continue;
                }
                0x03 => {
                    out.push(0xFF);
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(data[i]);
        i += 1;
    }
    out
}

/// Read a variable-length unsigned integer (little-endian) up to 8 bytes.
fn read_var_uint<R: Read>(r: &mut R, len: usize) -> Result<u64> {
    if len == 0 {
        return Ok(0);
    }
    if len > 8 {
        return Err(Error::SteamIdTooLong);
    }
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf[..len])?;
    Ok(u64::from_le_bytes(buf))
}

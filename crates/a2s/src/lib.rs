pub mod errors;
pub mod info;
#[cfg(feature = "async")]
pub mod nonblocking;
pub mod players;
pub mod rules;

use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::ops::Deref;
use std::time::Duration;

use bstr::BString;
use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use bzip2::read::BzDecoder;
use crc::crc32;

use crate::errors::Error;
use crate::errors::Result;

pub(crate) const SINGLE_PACKET: i32 = -1;
pub(crate) const MULTI_PACKET: i32 = -2;
pub(crate) const MAX_CHALLENGE_RETRIES: usize = 3;

/// Response header bytes.
pub const HEADER_CHALLENGE: u8 = b'A';
pub const HEADER_INFO: u8 = b'I';
pub const HEADER_PLAYER: u8 = b'D';
pub const HEADER_RULES: u8 = b'E';

// Offsets
pub(crate) const OFS_HEADER: usize = 0;
pub(crate) const OFS_SP_PAYLOAD: usize = 4;
pub(crate) const OFS_MP_ID: usize = 4;
pub(crate) const OFS_MP_SS_TOTAL: usize = 8;
pub(crate) const OFS_MP_SS_NUMBER: usize = 9;
pub(crate) const OFS_MP_SS_SIZE: usize = 10;
pub(crate) const OFS_MP_SS_BZ2_SIZE: usize = 12;
pub(crate) const OFS_MP_SS_BZ2_CRC: usize = 16;
pub(crate) const OFS_MP_SS_PAYLOAD: usize = OFS_MP_SS_BZ2_SIZE;
pub(crate) const OFS_MP_SS_PAYLOAD_BZ2: usize = OFS_MP_SS_BZ2_CRC + 4;

macro_rules! read_buffer_offset {
    ($buf:expr, $offset:expr, i8) => {
        $buf[$offset].into()
    };
    ($buf:expr, $offset:expr, u8) => {
        $buf[$offset].into()
    };
    ($buf:expr, $offset:expr, i16) => {
        i16::from_le_bytes([$buf[$offset], $buf[$offset + 1]])
    };
    ($buf:expr, $offset:expr, u16) => {
        u16::from_le_bytes([$buf[$offset], $buf[$offset + 1]])
    };
    ($buf:expr, $offset:expr, i32) => {
        i32::from_le_bytes([
            $buf[$offset],
            $buf[$offset + 1],
            $buf[$offset + 2],
            $buf[$offset + 3],
        ])
    };
    ($buf:expr, $offset:expr, u32) => {
        u32::from_le_bytes([
            $buf[$offset],
            $buf[$offset + 1],
            $buf[$offset + 2],
            $buf[$offset + 3],
        ])
    };
    ($buf:expr, $offset:expr, i64) => {
        i64::from_le_bytes([
            $buf[$offset],
            $buf[$offset + 1],
            $buf[$offset + 2],
            $buf[$offset + 3],
            $buf[$offset + 4],
            $buf[$offset + 5],
            $buf[$offset + 6],
            $buf[$offset + 7],
        ])
    };
    ($buf:expr, $offset:expr, u64) => {
        u64::from_le_bytes([
            $buf[$offset],
            $buf[$offset + 1],
            $buf[$offset + 2],
            $buf[$offset + 3],
            $buf[$offset + 4],
            $buf[$offset + 5],
            $buf[$offset + 6],
            $buf[$offset + 7],
        ])
    };
}

#[cfg(feature = "async")]
pub(crate) use read_buffer_offset;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Serialize;

/// A Steam Application ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct AppId(pub u16);

impl AppId {
    /// The Ship (2400)
    pub const THE_SHIP: Self = Self(2400);
}

/// A 64-bit Steam ID identifying a user or server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SteamId(pub u64);

/// A 64-bit Game ID. The low 24 bits contain a more accurate App ID
/// than the 16-bit [`AppId`] field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct GameId(pub u64);

#[cfg(feature = "arbitrary")]
pub(crate) fn arbitrary_bstring(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<BString> {
    let bytes: Vec<u8> = arbitrary::Arbitrary::arbitrary(u)?;
    Ok(BString::new(bytes))
}

#[cfg(feature = "arbitrary")]
pub(crate) fn arbitrary_option_bstring(
    u: &mut arbitrary::Unstructured<'_>,
) -> arbitrary::Result<Option<BString>> {
    if arbitrary::Arbitrary::arbitrary(u)? {
        Ok(Some(arbitrary_bstring(u)?))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct DeOptions {
    pub the_ship: bool,
}

impl DeOptions {
    pub fn from_app_id(app_id: AppId) -> Self {
        Self {
            the_ship: app_id == AppId::THE_SHIP,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PacketFragment {
    pub number: u8,
    pub payload: Vec<u8>,
}

pub struct A2SClient {
    socket: UdpSocket,
    max_size: usize,
    pub(crate) de_options: DeOptions,
}

impl A2SClient {
    pub fn new(timeout: Duration) -> Result<A2SClient> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;

        socket.set_read_timeout(Some(timeout))?;
        socket.set_write_timeout(Some(timeout))?;

        Ok(A2SClient {
            socket,
            max_size: 1400,
            de_options: DeOptions::default(),
        })
    }

    pub fn max_size(&mut self, size: usize) -> &mut Self {
        self.max_size = size;
        self
    }

    #[deprecated(since = "0.6.2", note = "use de_options")]
    pub fn app_id(&mut self, app_id: AppId) -> &mut Self {
        self.de_options = DeOptions::from_app_id(app_id);
        self
    }

    pub fn de_options(&mut self, de_options: DeOptions) -> &mut Self {
        self.de_options = de_options;
        self
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<&mut Self> {
        self.socket.set_read_timeout(Some(timeout))?;
        self.socket.set_write_timeout(Some(timeout))?;
        Ok(self)
    }

    #[doc(hidden)]
    pub fn send<A: ToSocketAddrs>(&self, payload: &[u8], addr: A) -> Result<Vec<u8>> {
        self.socket.send_to(payload, addr)?;

        let mut data = vec![0; self.max_size];

        let read = self.socket.recv(&mut data)?;
        data.truncate(read);

        let header = read_buffer_offset!(&data, OFS_HEADER, i32);

        if header == SINGLE_PACKET {
            Ok(data[OFS_SP_PAYLOAD..].to_vec())
        } else if header == MULTI_PACKET {
            let id = read_buffer_offset!(&data, OFS_MP_ID, i32);
            let total_packets: usize = data[OFS_MP_SS_TOTAL].into();
            let switching_size: usize = read_buffer_offset!(&data, OFS_MP_SS_SIZE, u16).into();

            if (switching_size > self.max_size) || (total_packets > 32) {
                return Err(Error::MultiPacketTooLarge);
            }

            let mut packets: Vec<PacketFragment> = Vec::with_capacity(0);
            packets.try_reserve(total_packets)?;
            packets.push(PacketFragment {
                number: data[OFS_MP_SS_NUMBER],
                payload: Vec::from(&data[OFS_MP_SS_PAYLOAD + 4..]),
            });

            loop {
                let mut data: Vec<u8> = Vec::with_capacity(0);
                data.try_reserve(switching_size)?;
                data.resize(switching_size, 0);

                let read = self.socket.recv(&mut data)?;
                data.truncate(read);

                if data.len() <= 9 {
                    return Err(Error::PacketTooShort {
                        expected: 10,
                        actual: data.len(),
                    });
                }

                let packet_id = read_buffer_offset!(&data, OFS_MP_ID, i32);

                if packet_id != id {
                    return Err(Error::MismatchPacketId);
                }

                if id as u32 & 0x80000000 == 0 {
                    packets.push(PacketFragment {
                        number: data[OFS_MP_SS_NUMBER],
                        payload: Vec::from(&data[OFS_MP_SS_PAYLOAD..]),
                    });
                } else {
                    packets.push(PacketFragment {
                        number: data[OFS_MP_SS_NUMBER],
                        payload: Vec::from(&data[OFS_MP_SS_PAYLOAD_BZ2..]),
                    });
                }

                if packets.len() == total_packets {
                    break;
                }
            }

            packets.sort_by_key(|p| p.number);

            let mut aggregation = Vec::with_capacity(0);
            aggregation.try_reserve(total_packets * self.max_size)?;

            for p in packets {
                aggregation.extend(p.payload);
            }

            if id as u32 & 0x80000000 != 0 {
                let decompressed_size = read_buffer_offset!(&data, OFS_MP_SS_BZ2_SIZE, u32);
                let checksum = read_buffer_offset!(&data, OFS_MP_SS_BZ2_CRC, u32);

                if decompressed_size > (1024 * 1024) {
                    return Err(Error::InvalidBz2Size);
                }

                let mut decompressed = Vec::with_capacity(0);
                decompressed.try_reserve(decompressed_size as usize)?;
                decompressed.resize(decompressed_size as usize, 0);

                BzDecoder::new(aggregation.deref()).read_exact(&mut decompressed)?;

                if crc32::checksum_ieee(&decompressed) != checksum {
                    return Err(Error::ChecksumMismatch);
                }

                Ok(decompressed)
            } else {
                Ok(aggregation)
            }
        } else {
            Err(Error::UnexpectedHeader {
                expected: SINGLE_PACKET as u8,
                actual: data[0],
            })
        }
    }

    #[doc(hidden)]
    pub fn do_challenge_request<A: ToSocketAddrs>(
        &self,
        addr: A,
        header: &[u8],
    ) -> Result<Vec<u8>> {
        let packet = Vec::with_capacity(9);
        let mut packet = Cursor::new(packet);

        packet.write_all(header)?;
        packet.write_i32::<LittleEndian>(-1)?;

        let mut data = self.send(packet.get_ref(), &addr)?;

        for _ in 0..MAX_CHALLENGE_RETRIES {
            if data.first() != Some(&HEADER_CHALLENGE) {
                return Ok(data);
            }

            let mut cursor = Cursor::new(&data);
            cursor.read_u8()?; // skip challenge header
            let challenge = cursor.read_i32::<LittleEndian>()?;

            packet.set_position(5);
            packet.write_i32::<LittleEndian>(challenge)?;
            data = self.send(packet.get_ref(), &addr)?;
        }

        Ok(data)
    }
}

pub(crate) trait ReadCString: Read {
    fn read_cstring(&mut self) -> Result<BString> {
        let mut buf = Vec::with_capacity(256);
        while let Ok(byte) = self.read_u8() {
            if byte == 0 {
                break;
            }

            buf.push(byte);
        }

        Ok(BString::new(buf))
    }
}

/// Implement ReadCString for all types that implement Read
impl<R: Read + ?Sized> ReadCString for R {}

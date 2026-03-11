use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::ops::Deref;
use std::time::Duration;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use bzip2::read::BzDecoder;
use crc::crc32;
use tokio::net::ToSocketAddrs;
use tokio::net::UdpSocket;
use tokio::time;

use crate::DeOptions;
use crate::PacketFragment;
use crate::errors::Error;
use crate::errors::Result;
use crate::info::INFO_REQUEST;
use crate::info::Info;
use crate::players::PLAYER_REQUEST;
use crate::players::Player;
use crate::read_buffer_offset;
use crate::rules::RULES_REQUEST;
use crate::rules::Rule;

macro_rules! future_timeout {
    ($timeout:expr, $future:expr) => {
        match time::timeout($timeout, $future).await {
            Ok(value) => value,
            Err(_) => return Err(Error::ErrTimeout),
        }
    };
}

pub struct A2SClient {
    socket: UdpSocket,
    timeout: Duration,
    max_size: usize,
    de_options: DeOptions,
}

impl A2SClient {
    pub async fn new() -> Result<A2SClient> {
        Ok(A2SClient {
            socket: UdpSocket::bind("0.0.0.0:0").await?,
            timeout: Duration::new(5, 0),
            max_size: 1400,
            de_options: DeOptions::default(),
        })
    }

    pub fn max_size(&mut self, size: usize) -> &mut Self {
        self.max_size = size;
        self
    }

    #[deprecated(since = "0.6.2", note = "use de_options")]
    pub fn app_id(&mut self, app_id: u16) -> &mut Self {
        self.de_options = DeOptions::from_app_id(app_id);
        self
    }

    pub fn de_options(&mut self, de_options: DeOptions) -> &mut Self {
        self.de_options = de_options;
        self
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<&mut Self> {
        if timeout == Duration::ZERO {
            return Err(Error::Other("attempted to set timeout to 0"));
        }
        self.timeout = timeout;
        Ok(self)
    }

    #[doc(hidden)]
    pub async fn send<A: ToSocketAddrs>(&self, payload: &[u8], addr: A) -> Result<Vec<u8>> {
        future_timeout!(self.timeout, self.socket.send_to(payload, addr))?;

        let mut data = vec![0; self.max_size];

        let read = future_timeout!(self.timeout, self.socket.recv(&mut data))?;
        data.truncate(read);

        if data.len() < 4 {
            return Err(Error::InvalidResponse);
        }

        let header = read_buffer_offset!(&data, crate::OFS_HEADER, i32);

        if header == crate::SINGLE_PACKET {
            Ok(data[crate::OFS_SP_PAYLOAD..].to_vec())
        } else if header == crate::MULTI_PACKET {
            let id = read_buffer_offset!(&data, crate::OFS_MP_ID, i32);
            let total_packets: usize = data[crate::OFS_MP_SS_TOTAL].into();
            let switching_size: usize =
                read_buffer_offset!(&data, crate::OFS_MP_SS_SIZE, u16).into();

            if (switching_size > self.max_size) || (total_packets > 32) {
                return Err(Error::InvalidResponse);
            }

            let mut packets: Vec<PacketFragment> = Vec::with_capacity(0);
            packets.try_reserve(total_packets)?;
            packets.push(PacketFragment {
                number: data[crate::OFS_MP_SS_NUMBER],
                payload: Vec::from(&data[crate::OFS_MP_SS_PAYLOAD + 4..]),
            });

            loop {
                let mut data: Vec<u8> = Vec::with_capacity(0);
                data.try_reserve(switching_size)?;
                data.resize(switching_size, 0);

                let read = future_timeout!(self.timeout, self.socket.recv(&mut data))?;
                data.truncate(read);

                if data.len() <= 9 {
                    Err(Error::InvalidResponse)?
                }

                let packet_id = read_buffer_offset!(&data, crate::OFS_MP_ID, i32);

                if packet_id != id {
                    return Err(Error::MismatchID);
                }

                if id as u32 & 0x80000000 == 0 {
                    packets.push(PacketFragment {
                        number: data[crate::OFS_MP_SS_NUMBER],
                        payload: Vec::from(&data[crate::OFS_MP_SS_PAYLOAD..]),
                    });
                } else {
                    packets.push(PacketFragment {
                        number: data[crate::OFS_MP_SS_NUMBER],
                        payload: Vec::from(&data[crate::OFS_MP_SS_PAYLOAD_BZ2..]),
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
                let decompressed_size =
                    read_buffer_offset!(&data, crate::OFS_MP_SS_BZ2_SIZE, u32);
                let checksum = read_buffer_offset!(&data, crate::OFS_MP_SS_BZ2_CRC, u32);

                if decompressed_size > (1024 * 1024) {
                    return Err(Error::InvalidBz2Size);
                }

                let mut decompressed = Vec::with_capacity(0);
                decompressed.try_reserve(decompressed_size as usize)?;
                decompressed.resize(decompressed_size as usize, 0);

                BzDecoder::new(aggregation.deref()).read_exact(&mut decompressed)?;

                if crc32::checksum_ieee(&decompressed) != checksum {
                    return Err(Error::CheckSumMismatch);
                }

                Ok(decompressed)
            } else {
                Ok(aggregation)
            }
        } else {
            Err(Error::InvalidResponse)
        }
    }

    #[doc(hidden)]
    pub async fn do_challenge_request<A: ToSocketAddrs>(
        &self,
        addr: A,
        header: &[u8],
    ) -> Result<Vec<u8>> {
        let packet = Vec::with_capacity(9);
        let mut packet = Cursor::new(packet);

        packet.write_all(header)?;
        packet.write_i32::<LittleEndian>(-1)?;

        let data = self.send(packet.get_ref(), &addr).await?;
        let mut data = Cursor::new(data);

        let header = data.read_u8()?;
        if header != b'A' {
            return Err(Error::InvalidResponse);
        }

        let challenge = data.read_i32::<LittleEndian>()?;

        packet.set_position(5);
        packet.write_i32::<LittleEndian>(challenge)?;
        let data = self.send(packet.get_ref(), &addr).await?;

        Ok(data)
    }

    pub async fn info<A: ToSocketAddrs>(&self, addr: A) -> Result<Info> {
        let response = self.send(&INFO_REQUEST, &addr).await?;

        let mut packet = Cursor::new(&response);

        let header = packet.read_u8()?;
        if header == b'A' {
            let challenge = packet.read_i32::<LittleEndian>()?;

            let mut query = Vec::with_capacity(29);
            query.write_all(&INFO_REQUEST)?;
            query.write_i32::<LittleEndian>(challenge)?;

            let data = self.send(&query, addr).await?;
            Info::from_reader(data.as_slice())
        } else {
            Info::from_reader(response.as_slice())
        }
    }

    pub async fn players<A: ToSocketAddrs>(&self, addr: A) -> Result<Vec<Player>> {
        let data = self.do_challenge_request(addr, &PLAYER_REQUEST).await?;
        Player::from_reader(data.as_slice(), &self.de_options)
    }

    pub async fn rules<A: ToSocketAddrs>(&self, addr: A) -> Result<Vec<Rule>> {
        let data = self.do_challenge_request(addr, &RULES_REQUEST).await?;
        Rule::from_reader(data.as_slice())
    }
}

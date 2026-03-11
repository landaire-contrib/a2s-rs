use std::io::Cursor;
use std::io::Read;
use std::net::ToSocketAddrs;

use bstr::BString;
use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Serialize;

use crate::A2SClient;
use crate::DeOptions;
use crate::HEADER_PLAYER;
use crate::ReadCString;
use crate::errors::Error;
use crate::errors::Result;

#[doc(hidden)]
pub const PLAYER_REQUEST: [u8; 5] = [0xff, 0xff, 0xff, 0xff, 0x55];

#[derive(Debug, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[non_exhaustive]
pub struct Player {
    /// Index of player chunk starting from 0.
    /// This seems to be always 0?
    pub index: u8,

    /// Name of the player.
    #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::arbitrary_bstring))]
    pub name: BString,

    /// Player's score (usually "frags" or "kills".)
    pub score: i32,

    /// Time (in seconds) player has been connected to the server.
    pub duration: f32,

    /// The Ship additional player info
    pub the_ship: Option<TheShipPlayer>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[non_exhaustive]
pub struct TheShipPlayer {
    pub deaths: u32,

    pub money: u32,
}

impl Player {
    #[deprecated(since = "0.6.2", note = "use from_reader")]
    pub fn from_cursor(data: Cursor<Vec<u8>>, app_id: u16) -> Result<Vec<Self>> {
        Self::from_reader(data, &DeOptions::from_app_id(app_id))
    }

    pub fn from_reader<R: Read>(mut data: R, options: &DeOptions) -> Result<Vec<Self>> {
        let header = data.read_u8()?;
        if header != HEADER_PLAYER {
            return Err(Error::UnexpectedHeader {
                expected: HEADER_PLAYER,
                actual: header,
            });
        }

        let player_count = data.read_u8()?;

        let mut players: Vec<Self> = Vec::with_capacity(player_count as usize);

        for _ in 0..player_count {
            players.push(Self {
                index: data.read_u8()?,
                name: data.read_cstring()?,
                score: data.read_i32::<LittleEndian>()?,
                duration: data.read_f32::<LittleEndian>()?,
                the_ship: if options.the_ship {
                    Some(TheShipPlayer {
                        deaths: data.read_u32::<LittleEndian>()?,
                        money: data.read_u32::<LittleEndian>()?,
                    })
                } else {
                    None
                },
            })
        }

        Ok(players)
    }
}

impl A2SClient {
    pub fn players<A: ToSocketAddrs>(&self, addr: A) -> Result<Vec<Player>> {
        let data = self.do_challenge_request(addr, &PLAYER_REQUEST)?;
        Player::from_reader(data.as_slice(), &self.de_options)
    }
}

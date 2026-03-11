use std::convert::TryFrom;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
#[cfg(not(feature = "async"))]
use std::net::ToSocketAddrs;

#[cfg(feature = "async")]
use tokio::net::ToSocketAddrs;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Serialize;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use crate::A2SClient;
use crate::ReadCString;
use crate::errors::Error;
use crate::errors::Result;

#[doc(hidden)]
pub const INFO_REQUEST: [u8; 25] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0x54, 0x53, 0x6F, 0x75, 0x72, 0x63, 0x65, 0x20, 0x45, 0x6E, 0x67, 0x69,
    0x6E, 0x65, 0x20, 0x51, 0x75, 0x65, 0x72, 0x79, 0x00,
];

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TheShip {
    /// Indicates the game mode
    pub mode: TheShipMode,

    /// The number of witnesses necessary to have a player arrested.
    pub witnesses: u8,

    /// Time (in seconds) before a player is arrested while being witnessed.
    pub duration: u8,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[repr(u8)]
pub enum TheShipMode {
    Hunt = 0,
    Elimination = 1,
    Duel = 2,
    Deathmatch = 3,
    VIPTeam = 4,
    TeamElimination = 5,
    Unknown = 255,
}

impl From<u8> for TheShipMode {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Hunt,
            1 => Self::Elimination,
            2 => Self::Duel,
            3 => Self::Deathmatch,
            4 => Self::VIPTeam,
            5 => Self::TeamElimination,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct ExtendedServerInfo {
    /// The server's game port number.
    /// Available if edf & 0x80 is true
    pub port: Option<u16>,

    /// Server's SteamID.
    /// Available if edf & 0x10 is true
    pub steam_id: Option<u64>,

    /// Tags that describe the game according to the server (for future use.)
    /// Available if edf & 0x20 is true
    pub keywords: Option<String>,

    /// The server's 64-bit GameID. If this is present, a more accurate AppID is present in the low 24 bits.
    /// The earlier AppID could have been truncated as it was forced into 16-bit storage.
    /// Avaialble if edf & 0x01 is true
    pub game_id: Option<u64>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SourceTVInfo {
    /// Spectator port number for SourceTV.
    pub port: u16,

    /// Name of the spectator server for SourceTV.
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[repr(u8)]
pub enum ServerType {
    Dedicated = b'd',
    NonDedicated = b'i',
    SourceTV = b'p',
}

impl TryFrom<u8> for ServerType {
    type Error = Error;
    fn try_from(val: u8) -> Result<Self> {
        match val {
            b'd' => Ok(Self::Dedicated),
            b'i' => Ok(Self::NonDedicated),
            b'p' => Ok(Self::SourceTV),
            _ => Err(Self::Error::Other("Invalid server type")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[repr(u8)]
pub enum ServerOS {
    Linux = b'l',
    Windows = b'w',
    Mac = b'm',
}

impl TryFrom<u8> for ServerOS {
    type Error = Error;

    fn try_from(val: u8) -> Result<Self> {
        match val {
            b'l' => Ok(Self::Linux),
            b'w' => Ok(Self::Windows),
            b'm' | b'o' => Ok(Self::Mac),
            _ => Err(Self::Error::Other("Invalid environment")),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Info {
    /// Protocol version used by the server.
    pub protocol: u8,

    /// Name of the server.
    pub name: String,

    /// Map the server has currently loaded.
    pub map: String,

    /// Name of the folder containing the game files.
    pub folder: String,

    /// Full name of the game.
    pub game: String,

    /// Steam Application ID of game.
    pub app_id: u16,

    /// Number of players on the server.
    pub players: u8,

    /// Maximum number of players the server reports it can hold.
    pub max_players: u8,

    /// Number of bots on the server.
    pub bots: u8,

    /// Indicates the type of server
    /// Rag Doll Kung Fu servers always return 0 for "Server type."
    pub server_type: ServerType,

    /// Indicates the operating system of the server
    pub server_os: ServerOS,

    /// Indicates whether the server requires a password
    pub visibility: bool,

    /// Specifies whether the server uses VAC
    pub vac: bool,

    /// These fields only exist in a response if the server is running The Ship
    pub the_ship: Option<TheShip>,

    /// Version of the game installed on the server.
    pub version: String,

    /// If present, this specifies which additional data fields will be included.
    pub edf: u8,

    pub extended_server_info: ExtendedServerInfo,

    /// Available if edf & 0x40 is true
    pub source_tv: Option<SourceTVInfo>,
}

impl Info {
    pub fn size_hint(&self) -> usize {
        // header(5) + protocol(1) + name+nul + map+nul + folder+nul + game+nul
        // + app_id(2) + players(1) + max_players(1) + bots(1) + server_type(1) + server_os(1)
        // + visibility(1) + vac(1) + version+nul
        let mut size = 5
            + 1
            + self.name.len()
            + 1
            + self.map.len()
            + 1
            + self.folder.len()
            + 1
            + self.game.len()
            + 1
            + 2
            + 1
            + 1
            + 1
            + 1
            + 1
            + 1
            + 1
            + self.version.len()
            + 1;

        if self.the_ship.is_some() {
            size += 3;
        }

        if self.edf != 0 {
            size += 1; // edf byte
        }
        if self.extended_server_info.port.is_some() {
            size += 2;
        }
        if self.extended_server_info.steam_id.is_some() {
            size += 8;
        }
        if let Some(keywords) = &self.extended_server_info.keywords {
            size += keywords.len() + 1;
        }
        if self.extended_server_info.game_id.is_some() {
            size += 8;
        }
        if let Some(source_tv) = &self.source_tv {
            size += 2 + source_tv.name.len() + 1;
        }

        size
    }

    pub fn write<W: Write>(&self, mut w: W) -> Result<()> {
        w.write_all(&[0xff, 0xff, 0xff, 0xff, 0x49])?;
        w.write_all(&[self.protocol])?;
        w.write_all(self.name.as_bytes())?;
        w.write_all(&[0])?;
        w.write_all(self.map.as_bytes())?;
        w.write_all(&[0])?;
        w.write_all(self.folder.as_bytes())?;
        w.write_all(&[0])?;
        w.write_all(self.game.as_bytes())?;
        w.write_all(&[0])?;
        w.write_all(&self.app_id.to_le_bytes())?;
        w.write_all(&[self.players, self.max_players, self.bots])?;
        w.write_all(&[self.server_type as u8])?;
        w.write_all(&[self.server_os as u8])?;
        w.write_all(&[if self.visibility { 1 } else { 0 }])?;
        w.write_all(&[if self.vac { 1 } else { 0 }])?;

        if let Some(the_ship) = &self.the_ship {
            w.write_all(&[the_ship.mode as u8, the_ship.witnesses, the_ship.duration])?;
        }

        w.write_all(self.version.as_bytes())?;
        w.write_all(&[0])?;

        if self.edf != 0 {
            w.write_all(&[self.edf])?;
        }

        if let Some(port) = &self.extended_server_info.port {
            w.write_all(&port.to_le_bytes())?;
        }
        if let Some(steam_id) = &self.extended_server_info.steam_id {
            w.write_all(&steam_id.to_le_bytes())?;
        }
        if let Some(keywords) = &self.extended_server_info.keywords {
            w.write_all(keywords.as_bytes())?;
            w.write_all(&[0])?;
        }
        if let Some(game_id) = &self.extended_server_info.game_id {
            w.write_all(&game_id.to_le_bytes())?;
        }

        if let Some(source_tv) = &self.source_tv {
            w.write_all(&source_tv.port.to_le_bytes())?;
            w.write_all(source_tv.name.as_bytes())?;
            w.write_all(&[0])?;
        }

        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.size_hint());
        self.write(&mut bytes)
            .expect("writing to Vec should not fail");
        bytes
    }

    #[deprecated(since = "0.6.2", note = "use from_reader")]
    pub fn from_cursor(data: Cursor<Vec<u8>>) -> Result<Self> {
        Self::from_reader(data)
    }

    pub fn from_reader<R: Read>(mut data: R) -> Result<Self> {
        if data.read_u8()? != 0x49u8 {
            return Err(Error::InvalidResponse);
        }

        let protocol = data.read_u8()?;
        let name = data.read_cstring()?;
        let map = data.read_cstring()?;
        let folder = data.read_cstring()?;
        let game = data.read_cstring()?;
        let app_id = data.read_u16::<LittleEndian>()?;
        let players = data.read_u8()?;
        let max_players = data.read_u8()?;
        let bots = data.read_u8()?;
        let server_type = ServerType::try_from(data.read_u8()?)?;
        let server_os = ServerOS::try_from(data.read_u8()?)?;
        let visibility = data.read_u8()? != 0;
        let vac = data.read_u8()? != 0;
        let the_ship = if app_id == 2400 {
            Some(TheShip {
                mode: TheShipMode::from(data.read_u8()?),
                witnesses: data.read_u8()?,
                duration: data.read_u8()?,
            })
        } else {
            None
        };
        let version = data.read_cstring()?;
        let edf = match data.read_u8() {
            Ok(val) => val,
            Err(err) => {
                if err.kind() != ErrorKind::UnexpectedEof {
                    return Err(Error::Io(err));
                } else {
                    0
                }
            }
        };
        let extended_server_info = ExtendedServerInfo {
            port: if edf & 0x80 != 0 {
                Some(data.read_u16::<LittleEndian>()?)
            } else {
                None
            },
            steam_id: if edf & 0x10 != 0 {
                Some(data.read_u64::<LittleEndian>()?)
            } else {
                None
            },
            keywords: if edf & 0x20 != 0 {
                Some(data.read_cstring()?)
            } else {
                None
            },
            game_id: if edf & 0x01 != 0 {
                Some(data.read_u64::<LittleEndian>()?)
            } else {
                None
            },
        };
        let source_tv = if edf & 0x40 != 0 {
            Some(SourceTVInfo {
                port: data.read_u16::<LittleEndian>()?,
                name: data.read_cstring()?,
            })
        } else {
            None
        };

        Ok(Info {
            protocol,
            name,
            map,
            folder,
            game,
            app_id,
            players,
            max_players,
            bots,
            server_type,
            server_os,
            visibility,
            vac,
            the_ship,
            version,
            edf,
            extended_server_info,
            source_tv,
        })
    }
}

impl A2SClient {
    #[cfg(feature = "async")]
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

    #[cfg(not(feature = "async"))]
    pub fn info<A: ToSocketAddrs>(&self, addr: A) -> Result<Info> {
        let response = self.send(&INFO_REQUEST, &addr)?;

        let mut packet = Cursor::new(&response);

        let header = packet.read_u8()?;
        if header == b'A' {
            let challenge = packet.read_i32::<LittleEndian>()?;

            let mut query = Vec::with_capacity(29);
            query.write_all(&INFO_REQUEST)?;
            query.write_i32::<LittleEndian>(challenge)?;

            let data = self.send(&query, addr)?;
            Info::from_reader(data.as_slice())
        } else {
            Info::from_reader(response.as_slice())
        }
    }
}

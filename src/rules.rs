use std::io::Cursor;
use std::io::Read;
use std::io::Write;
#[cfg(not(feature = "async"))]
use std::net::ToSocketAddrs;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

#[cfg(feature = "async")]
use tokio::net::ToSocketAddrs;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Serialize;

use crate::A2SClient;
use crate::ReadCString;
use crate::errors::Error;
use crate::errors::Result;

const RULES_REQUEST: [u8; 5] = [0xFF, 0xFF, 0xFF, 0xFF, 0x56];

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Rule {
    /// Name of the rule.
    pub name: String,

    /// Value of the rule.
    pub value: String,
}

impl Rule {
    pub fn size_hint(&self) -> usize {
        self.name.len() + 1 + self.value.len() + 1
    }

    pub fn write<W: Write>(&self, mut w: W) -> Result<()> {
        w.write_all(self.name.as_bytes())?;
        w.write_all(&[0])?;
        w.write_all(self.value.as_bytes())?;
        w.write_all(&[0])?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.size_hint());
        self.write(&mut bytes)
            .expect("writing to Vec should not fail");
        bytes
    }

    pub fn vec_size_hint(rules: &[Self]) -> usize {
        // header(5) + count(2) + sum of each rule
        5 + 2 + rules.iter().map(|r| r.size_hint()).sum::<usize>()
    }

    pub fn write_vec<W: Write>(rules: &[Self], mut w: W) -> Result<()> {
        w.write_all(&[0xff, 0xff, 0xff, 0xff, 0x45])?;
        w.write_all(&(rules.len() as u16).to_le_bytes())?;
        for rule in rules {
            rule.write(&mut w)?;
        }
        Ok(())
    }

    pub fn vec_to_bytes(rules: Vec<Self>) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::vec_size_hint(&rules));
        Self::write_vec(&rules, &mut bytes).expect("writing to Vec should not fail");
        bytes
    }

    #[deprecated(since = "0.6.2", note = "use from_reader")]
    pub fn from_cursor(data: Cursor<Vec<u8>>) -> Result<Vec<Self>> {
        Self::from_reader(data)
    }

    pub fn from_reader<R: Read>(mut data: R) -> Result<Vec<Self>> {
        if data.read_u8()? != 0x45 {
            return Err(Error::InvalidResponse);
        }

        let count = data.read_u16::<LittleEndian>()?;

        let mut rules: Vec<Rule> = Vec::with_capacity(count as usize);

        for _ in 0..count {
            rules.push(Rule {
                name: data.read_cstring()?,
                value: data.read_cstring()?,
            })
        }

        Ok(rules)
    }
}

impl A2SClient {
    #[cfg(feature = "async")]
    pub async fn rules<A: ToSocketAddrs>(&self, addr: A) -> Result<Vec<Rule>> {
        let data = self.do_challenge_request(addr, &RULES_REQUEST).await?;
        Rule::from_reader(data.as_slice())
    }

    #[cfg(not(feature = "async"))]
    pub fn rules<A: ToSocketAddrs>(&self, addr: A) -> Result<Vec<Rule>> {
        let data = self.do_challenge_request(addr, &RULES_REQUEST)?;
        Rule::from_reader(data.as_slice())
    }
}

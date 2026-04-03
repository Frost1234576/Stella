use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::io::{Read, Write};
use crate::binaries::jar::ClassInfo;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AST { /* ... */ } // NOT CODED YET - lowk dont know how this is gonna work

#[derive(Serialize, Deserialize, Debug)]
pub struct StellaV1 {
    pub classes: Vec<ClassInfo>,
    pub stella_asts: Vec<AST>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StellaV2 {
    pub classes: Vec<ClassInfo>,
    pub stella_asts: Vec<AST>,
    pub metadata: String, // Added in V2
}

// --- 2. The Versioned Container ---

#[derive(Debug)]
pub enum StellaData {
    V1(StellaV1),
    V2(StellaV2),
}

#[derive(Debug)]
pub struct StellaBinary {
    pub version: u8,
    pub data: StellaData,
}

// --- 3. Custom Serialization Logic ---

impl StellaBinary {
    pub fn save<W: Write>(&self, mut writer: W) -> bincode::Result<()> {
        // Write the version byte first
        bincode::serialize_into(&mut writer, &self.version)?;
        
        // Write the data corresponding to that version
        match &self.data {
            StellaData::V1(d) => bincode::serialize_into(writer, d),
            StellaData::V2(d) => bincode::serialize_into(writer, d),
        }
    }

    pub fn load<R: Read>(mut reader: R) -> bincode::Result<Self> {
        // Read the version byte first
        let version: u8 = bincode::deserialize_from(&mut reader)?;

        // Decide how to parse the rest based on the version
        let data = match version {
            1 => StellaData::V1(bincode::deserialize_from(reader)?),
            2 => StellaData::V2(bincode::deserialize_from(reader)?),
            _ => panic!("Unknown Stella version: {}", version),
        };

        Ok(StellaBinary { version, data })
    }
}
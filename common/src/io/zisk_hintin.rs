use std::path::Path;

use crate::io::{file_hintin::ZiskFileHintin, null_hintin::ZiskNullHintin, ZiskIO};
use anyhow::Result;

pub enum ZiskHintIOVariant {
    File(ZiskFileHintin),
    Null(ZiskNullHintin),
}

impl ZiskIO for ZiskHintIOVariant {
    fn read(&mut self) -> Vec<u8> {
        match self {
            ZiskHintIOVariant::File(file_hintin) => file_hintin.read(),
            ZiskHintIOVariant::Null(null_hintin) => null_hintin.read(),
        }
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        match self {
            ZiskHintIOVariant::File(file_hintin) => file_hintin.read_slice(slice),
            ZiskHintIOVariant::Null(null_hintin) => null_hintin.read_slice(slice),
        }
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        match self {
            ZiskHintIOVariant::File(file_hintin) => file_hintin.read_into(buffer),
            ZiskHintIOVariant::Null(null_hintin) => null_hintin.read_into(buffer),
        }
    }

    fn write_serialized(&mut self, data: &[u8]) {
        match self {
            ZiskHintIOVariant::File(file_hintin) => file_hintin.write_serialized(data),
            ZiskHintIOVariant::Null(null_hintin) => null_hintin.write_serialized(data),
        }
    }

    fn write_bytes(&mut self, data: &[u8]) {
        match self {
            ZiskHintIOVariant::File(file_hintin) => file_hintin.write_bytes(data),
            ZiskHintIOVariant::Null(null_hintin) => null_hintin.write_bytes(data),
        }
    }
}

pub struct ZiskHintin {
    io: ZiskHintIOVariant,
}

impl ZiskIO for ZiskHintin {
    fn read(&mut self) -> Vec<u8> {
        self.io.read()
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        self.io.read_slice(slice)
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        self.io.read_into(buffer)
    }

    fn write_serialized(&mut self, data: &[u8]) {
        self.io.write_serialized(data)
    }

    fn write_bytes(&mut self, data: &[u8]) {
        self.io.write_bytes(data)
    }
}

impl ZiskHintin {
    /// Create a null stdin (no input)
    pub fn null() -> Self {
        Self { io: ZiskHintIOVariant::Null(ZiskNullHintin) }
    }

    /// Create a file-based stdin
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self { io: ZiskHintIOVariant::File(ZiskFileHintin::new(path)?) })
    }
}

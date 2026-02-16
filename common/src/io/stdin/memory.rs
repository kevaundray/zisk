use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Mutex;

use crate::io::ZiskIO;

/// A memory-based implementation of ZiskStdin that reads from in-memory data.
pub struct ZiskMemoryStdin {
    data: Mutex<Vec<u8>>,
    cursor: Mutex<Cursor<Vec<u8>>>,
}

impl ZiskMemoryStdin {
    /// Create a new ZiskMemoryStdin from a vector of bytes.
    pub fn new(data: Vec<u8>) -> Self {
        let cursor = Mutex::new(Cursor::new(data.clone()));
        ZiskMemoryStdin { data: Mutex::new(data), cursor }
    }

    /// Create a new ZiskMemoryStdin from a string (UTF-8 encoded).
    pub fn from_string(data: String) -> Self {
        Self::new(data.into_bytes())
    }

    /// Create a new ZiskMemoryStdin from a slice of bytes.
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl ZiskIO for ZiskMemoryStdin {
    fn read_bytes(&self) -> Vec<u8> {
        // Return all the data
        self.data.lock().unwrap().clone()
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.read_exact(slice).expect("Failed to read slice from memory");
    }

    fn read_into(&self, buffer: &mut [u8]) {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.read_exact(buffer).expect("Failed to read into buffer from memory");
    }

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let mut cursor = self.cursor.lock().unwrap();
        bincode::serde::decode_from_std_read(&mut *cursor, bincode::config::standard())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize from memory: {}", e))
    }

    fn write<T: Serialize>(&self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serde::encode_into_std_write(data, &mut tmp, bincode::config::standard())
            .expect("Failed to serialize data into memory");
        self.data.lock().unwrap().extend_from_slice(&tmp);
        let mut cursor = self.cursor.lock().unwrap();
        cursor.get_mut().extend_from_slice(&tmp);
    }

    fn write_slice(&self, data: &[u8]) {
        let mut cursor = self.cursor.lock().unwrap();
        self.data.lock().unwrap().extend_from_slice(data);
        cursor.get_mut().extend_from_slice(data);
    }

    fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.data.lock().unwrap().as_slice())?;
        Ok(())
    }
}

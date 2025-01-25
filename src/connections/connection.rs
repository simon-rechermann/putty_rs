use super::errors::ConnectionError;
use clap::{ValueEnum};


/// A trait representing a generic connection (serial, SSH, etc.).
pub trait Connection {
    fn connect(&mut self) -> Result<(), ConnectionError>;
    fn disconnect(&mut self) -> Result<(), ConnectionError>;

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError>;
}

// enum used for UI
#[derive(Debug, ValueEnum)]
pub enum ConnectionType {
    Serial,
    Telnet,
    SSH
}

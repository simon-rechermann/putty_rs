use log::{info, error};
use std::fmt::format;
use std::io::{Write, Read}; // Add this to your imports
use std::net::TcpStream;

use crate::connections::{Connection, ConnectionError};

#[derive(Debug)]
pub struct TelnetConnection {
    address: String, 
    inner: Option<Box<TcpStream>>,
}

impl TelnetConnection {
    pub fn new(address: String) -> Self {
        TelnetConnection {
            address,
            inner: None
        }
    }
    /// Escapes IAC (255) bytes in the data by doubling it, as required by the Telnet protocol.
    fn escape_iac(data: &[u8]) -> Vec<u8> {
        let mut escaped = Vec::new();
        for &byte in data {
            escaped.push(byte);
            // If the byte is IAC (255), add an extra 255 to escape (double) it
            if byte == 255 {
                escaped.push(255);
            }
        }
        escaped
    }
}

impl Connection for TelnetConnection {
    fn connect(&mut self) -> Result<(), ConnectionError> {
        info!("Attempting to establish a TCP connection to: {}", self.address);

        let address_with_socket: String = format!("{}:{}", self.address, 23);
        let stream = TcpStream::connect(address_with_socket)?;

        info!("Successfully established a TCP connection to : {}", self.address);

        self.inner = Some(Box::new(stream));
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ConnectionError> {
        info!("Disconnecting from the server...");

        // To disconnect, we just drop the TcpStream, which automatically closes the connection.
        self.inner = None;

        info!("Successfully disconnected from the server.");
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(ref mut stream) = self.inner {
            // Telnet requires escaping special characters, like the IAC byte (255).
            let escaped_data = TelnetConnection::escape_iac(data);

            // Write escaped data to the stream
            stream.write_all(&escaped_data)?;
            stream.flush()?; // Ensure the data is actually sent
            Ok(escaped_data.len())
        } else {
            error!("Cannot write: TCP connection not established!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if let Some(ref mut stream) = self.inner {
            // Read data from the stream into the buffer.
            match stream.read(buffer) {
                Ok(bytes_read) => {
                    info!("Successfully read {} bytes from the connection.", bytes_read);
                    Ok(bytes_read)
                }
                Err(e) => {
                    Err(ConnectionError::IoError(e))
                }
            }
        } else {
            error!("Cannot write: TCP connection not established!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
}
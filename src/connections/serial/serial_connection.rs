use log::{error, info};
use serialport::SerialPort;
use std::time::Duration;

use crate::connections::{Connection, ConnectionError};

/// A struct that holds info about our serial connection.
#[derive(Debug)]
pub struct SerialConnection {
    port_path: String,
    baud_rate: u32,
    inner: Option<Box<dyn SerialPort>>,
}

impl SerialConnection {
    pub fn new(port_path: String, baud_rate: u32) -> Self {
        SerialConnection {
            port_path,
            baud_rate,
            inner: None,
        }
    }
}

impl Connection for SerialConnection {
    fn connect(&mut self) -> Result<(), ConnectionError> {
        info!("Attempting to open serial port: {}", self.port_path);

        // Use a short timeout (e.g. 10ms) so the read loop won't block forever.
        // This allows the thread to periodically check stop flags.
        let serial_port = serialport::new(&self.port_path, self.baud_rate)
            .timeout(Duration::from_millis(10))
            .open()?;

        info!("Successfully opened serial port: {}", self.port_path);

        self.inner = Some(serial_port);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if self.inner.is_some() {
            info!("Closing serial port: {}", self.port_path);
        }
        self.inner = None;
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(port) = self.inner.as_mut() {
            let bytes_written = port.write(data).map_err(ConnectionError::from)?;
            port.flush().map_err(ConnectionError::from)?;
            Ok(bytes_written)
        } else {
            error!("Cannot write: serial port not connected!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if let Some(port) = self.inner.as_mut() {
            // This will return Ok(0) if the read times out, or a real number of bytes if data arrives.
            // If there's an actual error, it returns Err(...).
            port.read(buffer).map_err(ConnectionError::from)
        } else {
            error!("Cannot read: serial port not connected!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
}

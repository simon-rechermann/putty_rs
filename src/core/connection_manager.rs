use log::{debug, error, info};
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{
    mpsc::{self, Sender, TryRecvError},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;

/// An event we can send to a connection's I/O thread.
enum IoEvent {
    Write(Vec<u8>),
    Stop,
}

/// Record of a single active connection:
/// - A thread handle for the I/O loop
/// - A Sender for IoEvent (writes + stop signals)
struct ConnectionIOThread {
    thread_handle: Option<thread::JoinHandle<()>>,
    tx: Sender<IoEvent>,
}

/// A handle to one specific connection, so the caller can write or stop it.
#[derive(Clone)]
pub struct ConnectionHandle {
    connection_manager: ConnectionManager,
    id: String,
}

/// The main `Session` that can hold multiple connections in a HashMap.
/// Each connection has its own dedicated I/O thread.
#[derive(Clone)]
pub struct ConnectionManager {
    /// Map "connection ID" -> ConnectionRecord
    inner: Arc<Mutex<HashMap<String, ConnectionIOThread>>>,
}

impl ConnectionManager {
    /// Create an empty Session.
    pub fn new() -> Self {
        ConnectionManager {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a new connection to this ConnectionManager.
    /// - `id`: A unique identifier (port name, e.g. "/dev/ttyUSB0")
    /// - `mut conn`: A *not yet connected* serial Connection
    /// - `on_byte`: A callback invoked on each received byte (with the connection `id`)
    ///
    /// Returns a `ConnectionHandle` so you can write/stop this connection.
    pub fn add_connection(
        &self,
        id: String,
        mut conn: Box<dyn Connection + Send>,
        mut on_byte: impl FnMut(u8) + Send + 'static,
    ) -> Result<ConnectionHandle, ConnectionError> {
        // 1) Actually connect the port
        conn.connect()?;

        // 2) Create a channel for IoEvent
        let (tx, rx) = mpsc::channel::<IoEvent>();

        // 3) Spawn the I/O thread
        let id_clone = id.clone();
        let thread_handle = thread::spawn(move || {
            info!("I/O thread started for connection '{}'.", id_clone);
            let mut buf = [0u8; 256];

            loop {
                // Check for any writes or Stop
                match rx.try_recv() {
                    Ok(IoEvent::Write(data)) => {
                        if let Err(e) = conn.write(&data) {
                            error!("Write error on '{}': {:?}", id_clone, e);
                        }
                    }
                    Ok(IoEvent::Stop) => {
                        info!("Stop received for '{}'. Exiting thread.", id_clone);
                        break;
                    }
                    Err(TryRecvError::Empty) => {
                        // No event
                    }
                    Err(TryRecvError::Disconnected) => {
                        info!("Channel disconnected for '{}'. Exiting.", id_clone);
                        break;
                    }
                }

                // Attempt to read
                match conn.read(&mut buf) {
                    Ok(0) => {
                        // no data
                    }
                    Ok(n) => {
                        for &byte in &buf[..n] {
                            on_byte(byte);
                        }
                        io::stdout().flush().ok();
                    }
                    Err(e) => {
                        debug!("Read error on '{}': {:?}", id_clone, e);
                        // Could be a timeout or real error
                    }
                }

                // Sleep briefly to avoid busy loop
                thread::sleep(Duration::from_millis(5));
            }

            // Cleanup
            let _ = conn.disconnect();
            info!("I/O thread ended for '{}'.", id_clone);
        });

        // 4) Store in our HashMap
        let record = ConnectionIOThread {
            thread_handle: Some(thread_handle),
            tx: tx.clone(),
        };
        {
            let mut map = self.inner.lock().unwrap();
            map.insert(id.clone(), record);
        }

        // 5) Return a handle
        Ok(ConnectionHandle {
            connection_manager: self.clone(),
            id,
        })
    }

    /// Write bytes to a specific connection by ID.
    pub fn write_bytes(&self, id: &str, data: &[u8]) -> Result<usize, ConnectionError> {
        let map = self.inner.lock().unwrap();
        if let Some(record) = map.get(id) {
            record
                .tx
                .send(IoEvent::Write(data.to_vec()))
                .map_err(|_| ConnectionError::Other("Channel closed".into()))?;
            Ok(data.len())
        } else {
            Err(ConnectionError::Other(format!(
                "No connection with id '{}'",
                id
            )))
        }
    }

    /// Stop one specific connection by ID (send Stop, join the thread, remove from map).
    pub fn stop_connection(&self, id: &str) -> Result<(), ConnectionError> {
        let mut map = self.inner.lock().unwrap();
        if let Some(mut record) = map.remove(id) {
            let _ = record.tx.send(IoEvent::Stop);
            if let Some(handle) = record.thread_handle.take() {
                let _ = handle.join();
            }
            Ok(())
        } else {
            Err(ConnectionError::Other(format!(
                "No connection with id '{}'",
                id
            )))
        }
    }
}

// -- ConnectionHandle methods --
// This is a small struct that references `Session` + an `id`.
impl ConnectionHandle {
    /// Writes data to *this* connection.
    pub fn write_bytes(&self, data: &[u8]) -> Result<usize, ConnectionError> {
        self.connection_manager.write_bytes(&self.id, data)
    }

    /// Stops *this* connection.
    pub fn stop(self) -> Result<(), ConnectionError> {
        self.connection_manager.stop_connection(&self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connections::connection::Connection;

    /// A simple mock `Connection` to test `ConnectionManager` logic without a real serial port.
    struct MockConnection {
        connected: bool,
        read_buffer: Vec<u8>,
        write_buffer: Vec<u8>,
    }

    impl MockConnection {
        fn new() -> Self {
            MockConnection {
                connected: false,
                read_buffer: vec![],
                write_buffer: vec![],
            }
        }
    }

    impl Connection for MockConnection {
        fn connect(&mut self) -> Result<(), ConnectionError> {
            self.connected = true;
            Ok(())
        }

        fn disconnect(&mut self) -> Result<(), ConnectionError> {
            self.connected = false;
            Ok(())
        }

        fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
            if !self.connected {
                return Err(ConnectionError::Other("Not connected".into()));
            }
            self.write_buffer.extend_from_slice(data);
            Ok(data.len())
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
            if !self.connected {
                return Err(ConnectionError::Other("Not connected".into()));
            }
            let n = self.read_buffer.len().min(buffer.len());
            buffer[..n].copy_from_slice(&self.read_buffer[..n]);
            self.read_buffer.drain(..n);
            Ok(n)
        }
    }

    #[test]
    fn test_write_and_stop() {
        let manager = ConnectionManager::new();
        let mock_connection = Box::new(MockConnection::new());
        let on_byte = |_byte: u8| {};

        let handle = manager
            .add_connection("mock".to_string(), mock_connection, on_byte)
            .expect("Failed to add mock connection");

        // Try writing
        let bytes_written = handle.write_bytes(b"Hello").unwrap();
        assert_eq!(bytes_written, 5);

        // Stop connection
        let result = handle.stop();
        assert!(result.is_ok(), "Stopping the connection should succeed");
    }
}

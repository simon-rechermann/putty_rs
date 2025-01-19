use log::{info, error};
use std::net::Ipv4Addr;
use std::net::TcpStream;

#[Derive(Debug)]
pub struct TelnetConnection {
    address: Ipv4Addr,
    inner: Option<Box<dyn TcpStream>>,
}

impl TelnetConnection {
    pub fn new(address: String) -> Self {
        TelnetConnection {
            address
        }
    }
}

impl Connection for TelnetConnection {
    fn connect(&mut self) -> Result<(), ConnectionError> {
        info!("Attempting to establish a TCP connection to: {}", self.address);

        let stream = TcpStream::connect(self.address)?;

        info!("Successfully established a TCP connection to : {}", self.address);

        self.inner = Some(stream);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ConnectionError> {
        todo!();
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        todo!();
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        todo!();
    }
}
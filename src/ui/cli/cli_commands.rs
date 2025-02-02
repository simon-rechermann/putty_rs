use clap::Parser;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use log::{info};
use std::net::Ipv4Addr;
use std::io::{self, Read};

use crate::connections::errors::ConnectionError;
use crate::connections::ConnectionType; 
use crate::connections::serial::SerialConnection;
use crate::connections::telnet::TelnetConnection;
use crate::core::connection_manager::{ConnectionHandle, ConnectionManager};

/// Enable raw mode via crossterm, throwing an error if it fails.
/// This disables line-buffering and echo on all supported platforms.
fn set_raw_mode() -> Result<(), ConnectionError> {
    enable_raw_mode()
        .map_err(|e| ConnectionError::Other(format!("Failed to enable raw mode: {}", e)))
}

/// Restore normal terminal mode.
/// crossterm internally remembers the previous mode and restores it.
fn restore_mode() {
    let _ = disable_raw_mode();
}
// 
/// Command-line arguments struct.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0")]
pub struct Args {
    /// Launch in GUI mode
    #[arg(long)]
    pub gui: bool,

    /// Serial port to open (default on UNIX: `/dev/pts/3`)
    /// If you're on Windows, you might specify something like `COM3`.
    #[arg(long)]
    pub port: Option<String>,

    #[arg(long, default_value_t = 115200)]
    pub baud: u32,

    #[arg(long)]
    pub connection_type: ConnectionType,

    #[arg(long)]
    pub address: Option<String>,
}

pub fn run_cli(args: Args) -> Result<(), ConnectionError> {
    match args.connection_type {
        ConnectionType::Serial => {
            if let Some(port) = args.port {
                info!("Opening serial port: {} at {} baud", port, args.baud);

                // 1) Create a ConnectionManager to manage one or more connections
                let connection_manager = ConnectionManager::new();

                // 2) Build a SerialConnection
                let conn = SerialConnection::new(port.clone(), args.baud);

                // 3) Provide a callback for incoming bytes
                //    In this example, we simply print them to stdout.
                let on_byte = move |byte: u8| {
                    // We ignore '_conn_id' since we only have one connection in CLI mode
                    if byte == b'\r' {
                        print!("\r");
                    } else {
                        print!("{}", byte as char);
                    }
                };

                // 4) Add the connection to the Session
                let handle: ConnectionHandle =
                    connection_manager.add_connection(port.clone(), Box::new(conn), on_byte)?;

                info!("Enable raw mode. Press Ctrl+A then 'x' to exit the program.");
                // Put terminal in raw mode (cross-platform with crossterm)
                set_raw_mode()?;

                let mut last_was_ctrl_a = false;
                let mut buf = [0u8; 1];

                // 5) Main loop reading from stdin, sending each char to the connection
                //    Because we're in raw mode, each typed character is read immediately.
                while io::stdin().read(&mut buf).is_ok() {
                    let ch = buf[0];

                    // If user typed Ctrl+A (ASCII 0x01), set a flag
                    if ch == 0x01 {
                        last_was_ctrl_a = true;
                        continue;
                    }

                    // If the previous character was Ctrl+A and the user typed 'x', exit
                    // and restore terminal mode
                    if last_was_ctrl_a && ch == b'x' {
                        restore_mode();
                        info!("Exiting...");
                        break;
                    } else {
                        last_was_ctrl_a = false;
                    }

                    // Optionally convert carriage return to something else
                    if ch == b'\r' {
                        let _ = handle.write_bytes(b"\r");
                    } else {
                        let _ = handle.write_bytes(&[ch]);
                    }
                }

                // 6) Stop the connection
                let _ = handle.stop();
                info!("Terminal mode restored.");
            } else {
                eprintln!("No --port argument provided.");
                eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
            }
        }
        ConnectionType::Telnet => {
            if let Some(address) = args.address {
                info!("Opening telnet connection to {:?}", address);
                // Create a ConnectionManager to manage one or more connections
                let connection_manager = ConnectionManager::new();
                info!("Connecting to address {}", address);

                let conn: TelnetConnection = TelnetConnection::new(address);

                // Provide a callback for incoming bytes
                //    In this example, we simply print them to stdout.
                let on_byte = move |byte: u8| {
                    // We ignore '_conn_id' since we only have one connection in CLI mode
                    if byte == b'\r' {
                        print!("\r");
                    } else {
                        print!("{}", byte as char);
                    }
                };

                // 4) Add the connection to the Session
                let _handle: ConnectionHandle = connection_manager.add_connection( format!("{:?}",args.connection_type), Box::new(conn), on_byte)?;
                // TODO continue point 4 and try it out
                todo!();

            } else {
                eprintln!("No all arguments for Telnet provided/valid.");
            }

        }

        ConnectionType::SSH => {
            todo!()
        }
    }
    Ok(())
}

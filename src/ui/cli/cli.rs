use clap::Parser;
use std::net::Ipv4Addr;
use log::info;
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;

use termios::*;
use crate::connections::ConnectionType; 
use crate::connections::errors::ConnectionError;
use crate::connections::serial::SerialConnection;
use crate::connections::telnet::TelnetConnection;
use crate::core::connection_manager::{ConnectionManager, ConnectionHandle};

/// Put stdin into raw mode so we read each keystroke immediately.
fn set_raw_mode() -> Result<Termios, ConnectionError> {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(stdin_fd)?;
    let original = termios.clone();

    // Disable canonical mode & echo
    termios.c_lflag &= !(ICANON | ECHO);

    // 1 byte at a time, no timeout
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

    tcsetattr(stdin_fd, TCSANOW, &termios)?;
    Ok(original)
}

fn restore_mode(original: Termios) {
    let stdin_fd = io::stdin().as_raw_fd();
    let _ = tcsetattr(stdin_fd, TCSANOW, &original);
}

/// Command-line arguments struct.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0")]
pub struct Args {
    /// Launch in GUI mode
    #[arg(long)]
    pub gui: bool,
    #[arg(long, default_value = "Telnet")]
    pub connection_type: ConnectionType,
    #[arg(long, default_value = "/dev/pts/3")]
    pub port: Option<String>,
    #[arg(long, default_value_t = 115200)]
    pub baud: u32,
}

pub fn run_cli(args: Args) -> Result<(), ConnectionError> {
    // 1) Create a Session to manage one or more connections
    let connection_manager = ConnectionManager::new();

    match args.connection_type {
        ConnectionType::Serial => {
            if let Some(port) = args.port {
                eprintln!("Opening telent port: {} at {} baud", port, args.baud);

                // 2) Build a SerialConnection
                let conn = SerialConnection::new(port.clone(), args.baud);

                // 3) Provide a callback for incoming bytes
                let on_byte = move |byte: u8| {
                    // We ignore '_conn_id' here because currently we only have one connection in CLI
                    if byte == b'\r' {
                        print!("\r");
                    } else {
                        print!("{}", byte as char);
                    }
                };

                // 4) Add the connection to the Session
                let handle: ConnectionHandle = connection_manager.add_connection(port.clone(), Box::new(conn), on_byte)?;
            } else {
                eprintln!("No --port argument provided.");
                eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
    }
        }
        ConnectionType::Telnet => {
            let conn: TelnetConnection = TelnetConnection::new(Ipv4Addr::new(127, 0,0,1));
        }

        ConnectionType::SSH => {
            todo!()
        }
    }

    // Put terminal in raw mode
    let original_mode = set_raw_mode()?;
    eprintln!("Raw mode enabled. Press Ctrl+A then 'x' to exit.");

    let mut last_was_ctrl_a = false;
    let mut buf = [0u8; 1];

    // 5) Main loop reading from stdin, sending each char to the connection
    while io::stdin().read(&mut buf).is_ok() {
        let ch = buf[0];

        if ch == 0x01 {
            last_was_ctrl_a = true;
            continue;
        }
        if last_was_ctrl_a && ch == b'x' {
            info!("Exiting...");
            break;
        } else {
            last_was_ctrl_a = false;
        }

        if ch == b'\r' {
            // Convert carriage return to newline if wanted here
            let _ = handle.write_bytes(b"\r");
        } else {
            let _ = handle.write_bytes(&[ch]);
        }
    }

        // 6) Stop the connection & restore terminal mode
        let _ = handle.stop();
        restore_mode(original_mode);
        eprintln!("Terminal mode restored.");


    Ok(())
}

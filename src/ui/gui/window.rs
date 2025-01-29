use eframe;
use eframe::egui;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::connections::serial::SerialConnection;
use crate::core::connection_manager::{ConnectionHandle, ConnectionManager};
use crate::ui::cli::cli_commands::Args;

pub fn launch_gui(args: Args) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "putty_rs GUI",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyGuiApp::new(args.port, args.baud)))),
    )
}

/// The main GUI application struct.
pub struct MyGuiApp {
    port: String,
    baud_str: String,

    /// A ConnectionManager that can hold multiple connections
    connection_manager: ConnectionManager,

    /// A map of "port" -> ConnectionHandle for each active connection
    connection_handles: HashMap<String, ConnectionHandle>,

    /// A buffer holding incoming data (shared by all ports currently)
    incoming_text: Arc<Mutex<String>>,

    /// The user “terminal” input buffer
    terminal_input: String,
    old_terminal_input: String,
}

impl Default for MyGuiApp {
    fn default() -> Self {
        MyGuiApp {
            port: "/dev/pts/3".to_owned(),
            baud_str: "115200".to_owned(),
            connection_manager: ConnectionManager::new(),
            connection_handles: HashMap::new(),
            incoming_text: Arc::new(Mutex::new(String::new())),
            terminal_input: String::new(),
            old_terminal_input: String::new(),
        }
    }
}

impl eframe::App for MyGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("putty_rs GUI");

            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.text_edit_singleline(&mut self.port);

                ui.label("Baud:");
                ui.text_edit_singleline(&mut self.baud_str);

                if ui.button("Connect").clicked() {
                    self.connect();
                }

                if ui.button("Disconnect").clicked() {
                    self.disconnect();
                }
            });

            ui.separator();

            // Show how many connections are active
            ui.label(format!(
                "Active connections: {}",
                self.connection_handles.len()
            ));

            // Output area
            ui.label("Terminal Output (all connections):");
            {
                let text_guard = self.incoming_text.lock().unwrap();
                let mut read_only_copy = text_guard.clone();
                drop(text_guard);

                egui::ScrollArea::vertical()
                    .id_salt("scroll_incoming_output")
                    .show(ui, |ui| {
                        ui.code_editor(&mut read_only_copy);
                    });
            }

            ui.separator();

            // Input area
            ui.label("Type here (new chars are sent to ALL active connections):");
            egui::ScrollArea::vertical()
                .id_salt("scroll_terminal_input")
                .show(ui, |ui| {
                    ui.code_editor(&mut self.terminal_input);
                });

            // If new typed characters arrived, send them out
            let old_len = self.old_terminal_input.len();
            let new_len = self.terminal_input.len();
            if new_len > old_len {
                let new_chars = self.terminal_input[old_len..new_len].to_string();
                self.send_chars(&new_chars);
            }
            self.old_terminal_input = self.terminal_input.clone();

            // Force continuous refresh
            ctx.request_repaint();
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Stop all active connections on exit
        for (_port, handle) in self.connection_handles.drain() {
            let _ = handle.stop();
        }
    }
}

impl MyGuiApp {
    pub fn new(port: Option<String>, baud: u32) -> Self {
        let mut s = Self::default();
        if let Some(p) = port {
            s.port = p;
        }
        s.baud_str = baud.to_string();
        s
    }

    /// Attempt to connect the specified port/baud
    fn connect(&mut self) {
        let baud = match self.baud_str.parse::<u32>() {
            Ok(b) => b,
            Err(_) => {
                error!("Invalid baud rate");
                return;
            }
        };

        let connection = SerialConnection::new(self.port.clone(), baud);

        let text_ref = self.incoming_text.clone();
        // Callback for every received byte from this port
        // (Currently we combine all ports' data in the same text buffer.)
        let on_byte = move |byte: u8| {
            let mut guard = text_ref.lock().unwrap();
            guard.push(byte as char);
        };

        // Add it to the connection manager
        match self.connection_manager.add_connection(
            self.port.clone(),
            Box::new(connection),
            on_byte,
        ) {
            Ok(handle) => {
                // Store the handle in our HashMap
                self.connection_handles.insert(self.port.clone(), handle);
                info!("Connected to {} at {}", self.port, baud);
            }
            Err(e) => {
                error!("Failed to connect: {:?}", e);
            }
        }
    }

    /// Disconnect the *current* port in the text field (if we have a handle for it).
    fn disconnect(&mut self) {
        // If a handle for `self.port` is in the map, remove it and stop.
        if let Some(handle) = self.connection_handles.remove(&self.port) {
            if let Err(e) = handle.stop() {
                error!("Error stopping connection {}: {:?}", self.port, e);
            } else {
                info!("Disconnected from {}", self.port);
            }
        } else {
            error!("No active connection found for '{}'", self.port);
        }
    }

    /// Send typed characters to *all* active connections
    /// (If you only want to send to one “active” port, you’d pick from the map.)
    fn send_chars(&mut self, chars: &str) {
        if chars.is_empty() {
            return;
        }
        for (port, handle) in &self.connection_handles {
            if let Err(e) = handle.write_bytes(chars.as_bytes()) {
                error!("Write error on port {}: {:?}", port, e);
            }
        }
    }
}

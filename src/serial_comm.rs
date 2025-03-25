use serialport::SerialPort;

use std::io::{self, BufRead, BufReader};
use std::time::Duration;

pub struct SerialObj {
    port: Option<Box<dyn SerialPort>>,
    baud_rate: u32,
}

impl SerialObj {
    pub fn new(serial_speed: u32) -> Self {
        SerialObj {
            port: None,
            baud_rate: serial_speed,
        }
    }

    pub fn get_ports() -> Vec<String> {
        serialport::available_ports()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .map(|p| p.port_name)
            .collect()
    }

    pub fn connect(&mut self, port_name: &str) -> io::Result<()> {
        let port = serialport::new(port_name, self.baud_rate)
            .timeout(Duration::from_millis(100))
            .open()?;
        self.port = Some(port);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.port.is_some()
    }

    pub fn get_data(&mut self) -> Option<String> {
        if let Some(ref mut port) = self.port {
            let mut reader = BufReader::new(port);
            let mut buffer = String::new();
            if reader.read_line(&mut buffer).is_ok() {
                return Some(buffer.trim().to_string());
            }
        }
        None
    }

    pub fn disconnect(&mut self) {
        self.port = None;
    }
}

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

fn main() {
    // Crea un objeto SerialObj con la velocidad de baudios deseada
    let mut serial = SerialObj::new(9600);

    // Lista de puertos disponibles
    let ports = SerialObj::get_ports();
    if ports.is_empty() {
        eprintln!("No se encontraron puertos seriales disponibles.");
        return;
    }

    // Conectar al primer puerto disponible
    let port_name = &ports[0];
    if let Err(e) = serial.connect(port_name) {
        eprintln!("Error al conectar al puerto {}: {}", port_name, e);
        return;
    }
    println!("Conectado al puerto: {}", port_name);

    // Bucle para leer y mostrar el último mensaje recibido
    loop {
        if let Some(message) = serial.get_data() {
            println!("Último mensaje recibido: {}", message);
        } else {
            println!("Esperando datos...");
        }

        // Puedes detener el bucle si lo deseas presionando Ctrl+C.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

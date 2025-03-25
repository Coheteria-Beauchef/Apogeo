use eframe::egui;
use serialport::SerialPort;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct App {
    latest_data: Arc<Mutex<(f32, f32, f32)>>,
}

impl App {
    fn new(latest_data: Arc<Mutex<(f32, f32, f32)>>) -> Self {
        Self { latest_data }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Datos Recibidos");

            let data = self.latest_data.lock().unwrap();
            ui.label(format!("Número 1: {:.2}", data.0));
            ui.label(format!("Número 2: {:.2}", data.1));
            ui.label(format!("Número 3: {:.2}", data.2));
        });

        ctx.request_repaint(); // Asegura que la UI se actualiza constantemente
    }
}

fn main() {
    let latest_data = Arc::new(Mutex::new((0.0, 0.0, 0.0)));
    let data_clone = Arc::clone(&latest_data);

    // Hilo para leer datos del puerto serie sin bloquear
    thread::spawn(move || {
        let ports = serialport::available_ports().expect("No se pudo obtener la lista de puertos");

        if ports.is_empty() {
            panic!("No se encontraron puertos disponibles.");
        }

        let port_name = &ports[0].port_name;
        println!("Usando el puerto: {}", port_name);

        let mut serial = serialport::new(port_name, 115200)
            .timeout(Duration::from_millis(100))
            .open()
            .expect("No se pudo abrir el puerto");

        let mut reader = BufReader::new(serial);
        let mut buffer = String::new();

        loop {
            if reader.read_line(&mut buffer).is_ok() {
                let trimmed = buffer.trim();
                let parts: Vec<&str> = trimmed.split(',').collect();

                if parts.len() == 3 {
                    let num1: f32 = parts[0].parse().unwrap_or(0.0);
                    let num2: f32 = parts[1].parse().unwrap_or(0.0);
                    let num3: f32 = parts[2].parse().unwrap_or(0.0);

                    if let Ok(mut data) = data_clone.lock() {
                        *data = (num1, num2, num3);
                    }
                }
                buffer.clear();
            }
            thread::sleep(Duration::from_millis(50)); // Evita uso excesivo de CPU
        }
    });

    // Crear la interfaz gráfica
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Monitor Serial",
        options,
        Box::new(|_cc| Box::new(App::new(latest_data))),
    )
    .expect("Error al ejecutar la interfaz");
}

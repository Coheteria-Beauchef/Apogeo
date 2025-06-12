use eframe::egui;
use egui::RichText;
use egui_plot::{Line, Plot, PlotPoints};
use image::GenericImageView;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct DataPoint {
    time: f64,         // tiempo en segundos
    thrust: f64,       // empuje
    temp_ambient: f64, // temperatura ambiente
    temp_nozzle: f64,  // temperatura tobera
}

struct App {
    last_data: Arc<Mutex<String>>,
    running: Arc<Mutex<bool>>,
    start_time: Instant,
    data_points: Arc<Mutex<Vec<DataPoint>>>,

    // Campos para la configuración
    port_name: String,
    baud_rate: u32,
    file_path: String,
    available_ports: Vec<String>,
    configured: bool,
    serial_thread: Option<thread::JoinHandle<()>>,

    // Campos para el logo
    logo_texture: Option<egui::TextureHandle>,
    logo_path: String,

    // Nuevos campos para carga de CSV
    csv_file_path: String,
    csv_data_loaded: bool,
    total_impulse: f64,
    current_mode: AppMode,

    // Nuevos campos para controlar los paneles
    show_csv_panel: bool,
    show_serial_panel: bool,
    error_message: String,
}

#[derive(PartialEq)]
enum AppMode {
    Configuration,
    LiveMonitoring,
    CsvViewer,
}

impl App {
    fn new() -> Self {
        // Escanear puertos disponibles
        let available_ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect();

        Self {
            last_data: Arc::new(Mutex::new("Esperando datos...".to_string())),
            running: Arc::new(Mutex::new(true)),
            start_time: Instant::now(),
            data_points: Arc::new(Mutex::new(Vec::new())),
            port_name: "COM9".to_string(),
            baud_rate: 115200,
            file_path: "datos.csv".to_string(),
            available_ports,
            configured: false,
            serial_thread: None,
            logo_texture: None,
            logo_path: "assets/logo.png".to_string(),
            csv_file_path: "datos.csv".to_string(),
            csv_data_loaded: false,
            total_impulse: 0.0,
            current_mode: AppMode::Configuration,
            show_csv_panel: false,
            show_serial_panel: false,
            error_message: String::new(),
        }
    }

    fn parse_time_to_seconds(time_str: &str) -> Option<f64> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() == 4 {
            if let (Ok(hours), Ok(minutes), Ok(seconds), Ok(millis)) = (
                parts[0].parse::<f64>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
                parts[3].parse::<f64>(),
            ) {
                return Some(hours * 3600.0 + minutes * 60.0 + seconds + millis / 1000.0);
            }
        }
        None
    }

    // Nueva función para formatear el tiempo transcurrido
    fn format_elapsed_time(&self) -> String {
        let elapsed = self.start_time.elapsed();
        let total_seconds = elapsed.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    fn load_csv_data(&mut self) -> Result<(), String> {
        let file = File::open(&self.csv_file_path)
            .map_err(|e| format!("Error al abrir el archivo: {}", e))?;

        let reader = BufReader::new(file);
        let mut data_points = Vec::new();
        let mut first_line = true;

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Error al leer línea: {}", e))?;

            if first_line {
                first_line = false;
                if line.contains("Tiempo") || line.contains("Empuje") {
                    continue;
                }
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                if let (Some(time), Ok(thrust), Ok(temp_ambient), Ok(temp_nozzle)) = (
                    Self::parse_time_to_seconds(parts[0].trim()),
                    parts[1].trim().parse::<f64>(),
                    parts[2].trim().parse::<f64>(),
                    parts[3].trim().parse::<f64>(),
                ) {
                    data_points.push(DataPoint {
                        time,
                        thrust,
                        temp_ambient,
                        temp_nozzle,
                    });
                }
            }
        }

        if data_points.is_empty() {
            return Err("No se encontraron datos válidos en el archivo CSV".to_string());
        }

        self.total_impulse = self.calculate_total_impulse(&data_points);
        *self.data_points.lock().unwrap() = data_points;
        self.csv_data_loaded = true;

        Ok(())
    }

    fn calculate_total_impulse(&self, data_points: &[DataPoint]) -> f64 {
        if data_points.len() < 2 {
            return 0.0;
        }

        let mut impulse = 0.0;
        for i in 1..data_points.len() {
            let dt = data_points[i].time - data_points[i - 1].time;
            let avg_thrust = (data_points[i].thrust + data_points[i - 1].thrust) / 2.0;
            impulse += avg_thrust * dt;
        }
        impulse
    }

    fn start_serial_thread(&mut self) {
        let last_data = Arc::clone(&self.last_data);
        let running = Arc::clone(&self.running);
        let data_points = Arc::clone(&self.data_points);
        let start_time = self.start_time;
        let port_name = self.port_name.clone();
        let baud_rate = self.baud_rate;
        let file_path = self.file_path.clone();

        let thread = thread::spawn(move || {
            let port = match serialport::new(&port_name, baud_rate)
                .timeout(Duration::from_millis(100))
                .open()
            {
                Ok(port) => port,
                Err(e) => {
                    let mut data = last_data.lock().unwrap();
                    *data = format!("Error al abrir el puerto: {}", e);
                    return;
                }
            };

            let file = match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
            {
                Ok(file) => file,
                Err(e) => {
                    let mut data = last_data.lock().unwrap();
                    *data = format!("Error al abrir el archivo: {}", e);
                    return;
                }
            };

            let mut file = file;
            let mut port = port;

            if file.metadata().unwrap().len() == 0 {
                let _ = writeln!(
                    file,
                    "Tiempo,Empuje,Temperatura Ambiente,Temperatura Tobera"
                );
            }

            let mut data = last_data.lock().unwrap();
            *data = "Conexión exitosa, esperando datos...".to_string();
            drop(data);

            loop {
                if !*running.lock().unwrap() {
                    break;
                }

                let mut buf = [0; 64];
                if let Ok(n) = port.read(&mut buf) {
                    if n > 0 {
                        let received = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                        let parts: Vec<&str> = received.split(',').collect();

                        if parts.len() == 3 {
                            let elapsed = start_time.elapsed();
                            let timestamp = format!(
                                "{:02}:{:02}:{:02}:{:03}",
                                elapsed.as_secs() / 3600,
                                (elapsed.as_secs() % 3600) / 60,
                                elapsed.as_secs() % 60,
                                elapsed.subsec_millis()
                            );

                            if let (Ok(thrust), Ok(temp_ambient), Ok(temp_nozzle)) = (
                                parts[0].trim().parse::<f64>(),
                                parts[1].trim().parse::<f64>(),
                                parts[2].trim().parse::<f64>(),
                            ) {
                                let _ = writeln!(
                                    file,
                                    "{},{},{},{}",
                                    timestamp, thrust, temp_ambient, temp_nozzle
                                );
                                let mut data = last_data.lock().unwrap();
                                *data = format!("{} | {} | {}", thrust, temp_ambient, temp_nozzle);

                                let mut data_points = data_points.lock().unwrap();
                                let time_seconds = elapsed.as_secs_f64();

                                if data_points.len() >= 100 {
                                    data_points.remove(0);
                                }
                                data_points.push(DataPoint {
                                    time: time_seconds,
                                    thrust,
                                    temp_ambient,
                                    temp_nozzle,
                                });
                            }
                        }
                    }
                }
            }
        });

        self.serial_thread = Some(thread);
    }

    fn show_config_window(&mut self, ui: &mut egui::Ui) {
        // Centrar todo el contenido
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // Logo si está disponible
            if let Some(texture) = &self.logo_texture {
                ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(150.0, 84.0)));
                ui.add_space(10.0);
            }

            // Título principal
            ui.heading(RichText::new("Dashboard de Análisis").size(24.0).strong());
            ui.add_space(30.0);

            // Mostrar mensaje de error si existe
            if !self.error_message.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.error_message);
                ui.add_space(10.0);
            }

            // Botones principales grandes y elegantes
            ui.vertical_centered(|ui| {
                ui.set_max_width(300.0);

                if ui
                    .add_sized(
                        [280.0, 60.0],
                        egui::Button::new(RichText::new("📊 Cargar archivo CSV").size(16.0)),
                    )
                    .clicked()
                {
                    self.show_csv_panel = true;
                    self.show_serial_panel = false;
                    self.error_message.clear();
                }

                ui.add_space(15.0);

                if ui
                    .add_sized(
                        [280.0, 60.0],
                        egui::Button::new(RichText::new("📡 Monitoreo en vivo").size(16.0)),
                    )
                    .clicked()
                {
                    self.show_serial_panel = true;
                    self.show_csv_panel = false;
                    self.error_message.clear();
                }
            });
        });

        // Mostrar paneles según corresponda
        if self.show_csv_panel {
            self.show_csv_panel_ui(ui);
        }

        if self.show_serial_panel {
            self.show_serial_panel_ui(ui);
        }
    }

    fn show_csv_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(30.0);
        ui.separator();
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.heading("Cargar datos desde CSV");
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label("Archivo:");
                ui.text_edit_singleline(&mut self.csv_file_path);
            });

            ui.add_space(15.0);

            ui.horizontal(|ui| {
                if ui
                    .add_sized([120.0, 35.0], egui::Button::new("Cargar"))
                    .clicked()
                {
                    match self.load_csv_data() {
                        Ok(()) => {
                            self.current_mode = AppMode::CsvViewer;
                            self.show_csv_panel = false;
                            self.error_message.clear();
                        }
                        Err(e) => {
                            self.error_message = format!("Error: {}", e);
                        }
                    }
                }

                if ui
                    .add_sized([120.0, 35.0], egui::Button::new("Cancelar"))
                    .clicked()
                {
                    self.show_csv_panel = false;
                    self.error_message.clear();
                }
            });
        });
    }

    fn show_serial_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(30.0);
        ui.separator();
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.heading("Configuración Serial");
            ui.add_space(20.0);

            ui.set_max_width(400.0);

            // Puerto
            ui.horizontal(|ui| {
                ui.label("Puerto:");
                egui::ComboBox::from_id_source("puerto")
                    .selected_text(&self.port_name)
                    .show_ui(ui, |ui| {
                        for port in &self.available_ports {
                            ui.selectable_value(&mut self.port_name, port.clone(), port);
                        }
                    });

                if ui.button("🔄").clicked() {
                    self.available_ports = serialport::available_ports()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| p.port_name)
                        .collect();
                }
            });

            ui.add_space(10.0);

            // Baud rate
            ui.horizontal(|ui| {
                ui.label("Velocidad:");
                egui::ComboBox::from_id_source("baud")
                    .selected_text(self.baud_rate.to_string())
                    .show_ui(ui, |ui| {
                        for &rate in &[9600, 19200, 38400, 57600, 115200, 230400] {
                            ui.selectable_value(&mut self.baud_rate, rate, rate.to_string());
                        }
                    });
            });

            ui.add_space(10.0);

            // Archivo
            ui.horizontal(|ui| {
                ui.label("Archivo:");
                ui.text_edit_singleline(&mut self.file_path);
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui
                    .add_sized([120.0, 35.0], egui::Button::new("Iniciar"))
                    .clicked()
                {
                    if !self.port_name.is_empty() && !self.file_path.is_empty() {
                        self.current_mode = AppMode::LiveMonitoring;
                        self.show_serial_panel = false;
                        self.start_time = Instant::now(); // Reiniciar el tiempo cuando se inicia el monitoreo
                        self.start_serial_thread();
                        self.error_message.clear();
                    } else {
                        self.error_message = "Por favor, complete todos los campos".to_string();
                    }
                }

                if ui
                    .add_sized([120.0, 35.0], egui::Button::new("Cancelar"))
                    .clicked()
                {
                    self.show_serial_panel = false;
                    self.error_message.clear();
                }
            });
        });
    }

    fn show_monitoring_ui(&mut self, ui: &mut egui::Ui) {
        let is_csv_mode = self.current_mode == AppMode::CsvViewer;

        // Encabezado con reloj alineado a la derecha para datos en vivo
        ui.horizontal(|ui| {
            if is_csv_mode {
                ui.heading("Análisis de datos CSV");
            } else {
                ui.heading("Datos en tiempo real");

                // Usar allocate_space para empujar el reloj hacia la derecha
                ui.allocate_space(ui.available_size() - egui::vec2(220.0, 0.0));

                // Reloj en tiempo real - alineado a la derecha
                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("⏱️ TIEMPO TRANSCURRIDO")
                                .size(12.0)
                                .color(egui::Color32::GRAY),
                        );
                        ui.label(
                            RichText::new(self.format_elapsed_time())
                                .size(24.0)
                                .strong()
                                .color(egui::Color32::from_rgb(0, 150, 255)),
                        );
                    });
                });
            }
        });

        // Información adicional
        if is_csv_mode {
            ui.horizontal(|ui| {
                ui.label(format!("Archivo: {}", self.csv_file_path));
                ui.label(format!("Impulso total: {:.2} N⋅s", self.total_impulse));
            });
        } else {
            let data = self.last_data.lock().unwrap();
            ui.label(format!("Últimos datos: {}", *data));
        }

        ui.horizontal(|ui| {
            if !is_csv_mode {
                if ui.button("Detener").clicked() {
                    *self.running.lock().unwrap() = false;
                }
            }

            if ui.button("Volver a configuración").clicked() {
                if !is_csv_mode {
                    *self.running.lock().unwrap() = false;
                    if let Some(thread) = self.serial_thread.take() {
                        let _ = thread.join();
                    }
                    *self.running.lock().unwrap() = true;
                }

                self.configured = false;
                self.csv_data_loaded = false;
                self.current_mode = AppMode::Configuration;
                self.show_csv_panel = false;
                self.show_serial_panel = false;
                self.data_points.lock().unwrap().clear();
                self.total_impulse = 0.0;
                self.error_message.clear();
                return;
            }

            if is_csv_mode {
                if ui.button("Exportar resumen").clicked() {
                    self.export_summary();
                }
            }
        });

        ui.separator();

        let data_points = self.data_points.lock().unwrap();
        let x_vals: Vec<f64> = if is_csv_mode {
            data_points.iter().map(|dp| dp.time).collect()
        } else {
            (0..data_points.len()).map(|i| i as f64).collect()
        };

        let thrust_vals: Vec<f64> = data_points.iter().map(|dp| dp.thrust).collect();
        let temp_ambient_vals: Vec<f64> = data_points.iter().map(|dp| dp.temp_ambient).collect();
        let temp_nozzle_vals: Vec<f64> = data_points.iter().map(|dp| dp.temp_nozzle).collect();

        let available_rect = ui.available_rect_before_wrap();
        let graph_width = (available_rect.width() - 20.0) / 2.0;
        let graph_height = (available_rect.height() - 100.0) / 2.0;

        ui.columns(2, |columns| {
            columns[0].group(|ui| {
                ui.set_min_size(egui::vec2(graph_width, graph_height * 2.0 + 20.0));
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Empuje").size(18.0).strong());
                });

                Plot::new("thrust_plot")
                    .width(graph_width - 20.0)
                    .height(graph_height * 2.0 - 20.0)
                    .x_axis_label(if is_csv_mode {
                        "Tiempo (s)"
                    } else {
                        "Muestras"
                    })
                    .y_axis_label("Empuje (N)")
                    .show(ui, |plot_ui| {
                        let line = Line::new(PlotPoints::from_iter(
                            x_vals.iter().zip(thrust_vals.iter()).map(|(&x, &y)| [x, y]),
                        ));
                        plot_ui.line(line);
                    });
            });

            columns[1].vertical(|ui| {
                ui.group(|ui| {
                    ui.set_min_size(egui::vec2(graph_width, graph_height));
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Temperatura Ambiente").size(16.0).strong());
                    });

                    Plot::new("temp_ambient_plot")
                        .width(graph_width - 20.0)
                        .height(graph_height - 40.0)
                        .x_axis_label(if is_csv_mode {
                            "Tiempo (s)"
                        } else {
                            "Muestras"
                        })
                        .y_axis_label("Temperatura (°C)")
                        .show(ui, |plot_ui| {
                            let line = Line::new(PlotPoints::from_iter(
                                x_vals
                                    .iter()
                                    .zip(temp_ambient_vals.iter())
                                    .map(|(&x, &y)| [x, y]),
                            ));
                            plot_ui.line(line);
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.set_min_size(egui::vec2(graph_width, graph_height));
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Temperatura Tobera").size(16.0).strong());
                    });

                    Plot::new("temp_nozzle_plot")
                        .width(graph_width - 20.0)
                        .height(graph_height - 40.0)
                        .x_axis_label(if is_csv_mode {
                            "Tiempo (s)"
                        } else {
                            "Muestras"
                        })
                        .y_axis_label("Temperatura (°C)")
                        .show(ui, |plot_ui| {
                            let line = Line::new(PlotPoints::from_iter(
                                x_vals
                                    .iter()
                                    .zip(temp_nozzle_vals.iter())
                                    .map(|(&x, &y)| [x, y]),
                            ));
                            plot_ui.line(line);
                        });
                });
            });
        });

        ui.separator();

        ui.group(|ui| {
            ui.set_min_width(available_rect.width() - 20.0);

            if is_csv_mode && !data_points.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Estadísticas del Análisis")
                            .size(16.0)
                            .strong(),
                    );
                });

                let max_thrust = thrust_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let avg_thrust = thrust_vals.iter().sum::<f64>() / thrust_vals.len() as f64;
                let duration = data_points.last().unwrap().time - data_points.first().unwrap().time;

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(format!("📊 Empuje máximo: {:.2} N", max_thrust));
                        ui.label(format!("📈 Empuje promedio: {:.2} N", avg_thrust));
                        ui.label(format!("⏱️ Duración: {:.2} s", duration));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(format!("🚀 Impulso total: {:.2} N⋅s", self.total_impulse));
                        ui.label(format!(
                            "⚡ Impulso específico: {:.2} s",
                            self.total_impulse / 9.81
                        ));
                        ui.label(format!("📋 Muestras totales: {}", data_points.len()));
                    });
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Estado del Sistema").size(16.0).strong());
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(format!("🔗 Puerto: {}", self.port_name));
                        ui.label(format!("⚡ Baud Rate: {}", self.baud_rate));
                        ui.label(format!("📁 Archivo: {}", self.file_path));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        if !data_points.is_empty() {
                            ui.label(format!("📊 Muestras actuales: {}", data_points.len()));
                            let last_point = data_points.last().unwrap();
                            ui.label(format!("🚀 Último empuje: {:.2} N", last_point.thrust));
                            ui.label(format!(
                                "🌡️ Temp. ambiente: {:.1}°C",
                                last_point.temp_ambient
                            ));
                            ui.label(format!("🔥 Temp. tobera: {:.1}°C", last_point.temp_nozzle));
                        } else {
                            ui.label("⏳ Esperando datos...");
                            ui.label("🔌 Verificar conexión serial");
                            ui.label("📡 Iniciando captura de datos...");
                        }
                    });
                });
            }
        });
    }

    fn export_summary(&self) {
        let data_points = self.data_points.lock().unwrap();
        if data_points.is_empty() {
            return;
        }

        let summary_file = "resumen_analisis.txt";
        if let Ok(mut file) = std::fs::File::create(summary_file) {
            let y1_vals: Vec<f64> = data_points.iter().map(|dp| dp.thrust).collect();
            let max_thrust = y1_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let avg_thrust = y1_vals.iter().sum::<f64>() / y1_vals.len() as f64;
            let duration = data_points.last().unwrap().time - data_points.first().unwrap().time;

            let _ = writeln!(file, "=== RESUMEN DEL ANÁLISIS ===");
            let _ = writeln!(file, "Archivo analizado: {}", self.csv_file_path);
            let _ = writeln!(file, "");
            let _ = writeln!(file, "ESTADÍSTICAS DE EMPUJE:");
            let _ = writeln!(file, "Empuje máximo: {:.2} N", max_thrust);
            let _ = writeln!(file, "Empuje promedio: {:.2} N", avg_thrust);
            let _ = writeln!(file, "");
            let _ = writeln!(file, "IMPULSO:");
            let _ = writeln!(file, "Impulso total: {:.2} N⋅s", self.total_impulse);
            let _ = writeln!(
                file,
                "Impulso específico: {:.2} s",
                self.total_impulse / 9.81
            );
            let _ = writeln!(file, "");
            let _ = writeln!(file, "DURACIÓN:");
            let _ = writeln!(file, "Duración total: {:.2} s", duration);
            let _ = writeln!(file, "Muestras totales: {}", data_points.len());
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Cargar logo
        if self.logo_texture.is_none() {
            if let Ok(img) = image::open(&self.logo_path) {
                let img_rgba = img.to_rgba8();
                let dimensions = img.dimensions();
                let pixels = img_rgba.into_raw();
                let image_data = egui::ColorImage::from_rgba_unmultiplied(
                    [dimensions.0 as usize, dimensions.1 as usize],
                    &pixels,
                );
                let texture =
                    ctx.load_texture("logo_texture", image_data, egui::TextureOptions::default());
                self.logo_texture = Some(texture);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.current_mode {
            AppMode::Configuration => {
                self.show_config_window(ui);
            }
            AppMode::LiveMonitoring | AppMode::CsvViewer => {
                self.show_monitoring_ui(ui);
            }
        });

        ctx.request_repaint();
    }
}
fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("Apogeo", options, Box::new(|_cc| Box::new(App::new()))).unwrap();
}

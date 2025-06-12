# 🚀 Apogeo

**Apogeo** es una herramienta desarrollada en **Rust** por el equipo **Cohetería Beauchef** (Universidad de Chile), diseñada para el análisis y monitoreo de datos telemétricos en misiones de cohetería.

---

## 🧩 Estructura del proyecto

```
Apogeo/
├── assets/                 # Recursos gráficos o de configuración
├── build/                  # Archivos de compilación o build scripts
├── src/                    # Código fuente en Rust
│   └── main.rs             # Punto de entrada
├── Cargo.toml              # Configuración del proyecto Rust
└── LICENSE                 # Licencia MIT
```

---

## 📦 Requisitos

- [Rust](https://www.rust-lang.org/tools/install) (última versión estable recomendada)
- [Cargo](https://doc.rust-lang.org/cargo/) (ya incluido con Rust)

---

## ⚙️ Instalación y compilación

```bash
# Clona el repositorio
git clone https://github.com/Coheteria-Beauchef/Apogeo.git
cd Apogeo

# Compila en modo release
cargo build --release

# O ejecuta directamente
cargo run --release
```

---

## 🚀 Uso

Ejecuta el binario generado (en `target/release/`):

```bash
./target/release/apogeo [opciones]
```

Puedes ver las opciones disponibles con:

```bash
./target/release/apogeo --help
```

Apogeo está diseñado para procesar datos telemétricos (como altitud, aceleración y orientación) y generar salidas útiles para monitoreo y análisis post-misión.

---

## 🛠️ Características principales

- Lectura y procesamiento de datos en tiempo real
- Análisis de parámetros críticos de vuelo (apogeo, velocidad, aceleración)
- Posibilidad de integración con sistemas externos de visualización o almacenamiento

---

## 📁 `assets/`

Contiene recursos utilizados por el programa, como gráficos, configuraciones u otros datos estáticos necesarios para la ejecución.

---

## 🧪 Tests

*(Esta sección se puede completar más adelante si se agregan pruebas automatizadas)*

---

## 📄 Licencia

Este proyecto está licenciado bajo los términos de la **Licencia MIT**. Consulta el archivo `LICENSE` para más detalles.

---

## 👥 Contribuir

¡Se agradecen las contribuciones!

1. Haz un fork del proyecto  
2. Crea una nueva rama con tu mejora o corrección  
3. Abre un Pull Request explicando tus cambios

---

## 📞 Contacto

Para consultas o colaboración, puedes contactar al equipo de [Cohetería Beauchef](https://github.com/Coheteria-Beauchef) a través del mismo repositorio o sus redes oficiales.

---

¡Gracias por tu interés en **Apogeo**! Tu aporte impulsa el desarrollo de la cohetería universitaria en Chile 🚀🇨🇱

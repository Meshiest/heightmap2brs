use std::{collections::VecDeque, sync::OnceLock};

use egui::{Color32, RichText, ScrollArea, Ui, mutex::Mutex};
use log::SetLoggerError;

struct EguiLogger;

static LOG: OnceLock<Mutex<VecDeque<(log::Level, String)>>> = OnceLock::new();

pub fn get_log() -> &'static Mutex<VecDeque<(log::Level, String)>> {
    LOG.get_or_init(|| Mutex::new(VecDeque::with_capacity(1000)))
}

impl log::Log for EguiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::STATIC_MAX_LEVEL
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut log = get_log().lock();

            let mut l: VecDeque<(log::Level, String)> = log.clone();
            l.push_back((record.level(), record.args().to_string()));
            if l.len() > 1000 {
                l.drain(0..1);
            }

            *log = l;
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&EguiLogger).map(|()| log::set_max_level(log::LevelFilter::Info))
}

pub fn draw(ui: &mut Ui) {
    let logs = get_log().lock();

    ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            logs.iter().for_each(|(level, string)| {
                let string_format = format!("[{}]: {}", level, string);

                ui.monospace(match level {
                    log::Level::Warn => RichText::new(string_format).color(Color32::YELLOW),
                    log::Level::Error => RichText::new(string_format).color(Color32::RED),
                    _ => RichText::new(string_format),
                });
            });
        });
}

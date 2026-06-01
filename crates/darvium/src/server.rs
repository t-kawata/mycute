// Darvium 観測サーバー — Axum + WebSocket によるシミュレーション可視化
//
// 本モジュールは `server` feature が有効な場合のみコンパイルされる。
// `darvium-observer` バイナリがエントリポイントとなり、本モジュールの
// `run()` 関数を呼び出して Web サーバーを起動する。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use tower_http::services::ServeDir;

use crate::event::DarviumEvent;
use crate::event_channel::BroadcastEventChannel;
use crate::simulation::{run_evaluation_simulation_with_channel, ReciprocitySimulatorConfig, SimulationParams};

// ============================================================
// 型定義
// ============================================================

/// シミュレーション制御コマンド。
pub enum SimCommand {
    Start(Box<ReciprocitySimulatorConfig>),
    Stop,
    Resume,
    Reset,
    UpdateParam(f64),
    UpdateChiefAttraction(f64),
    UpdateMinApproach(f64),
    /// 目標人口の更新（フロントエンドスライダーからのリアルタイム変更）。
    UpdateTargetPop(usize),
}

/// シミュレーション管理状態。
pub(crate) struct SharedSimState {
    cancel: Option<Arc<AtomicBool>>,
    last_config: Option<ReciprocitySimulatorConfig>,
}

/// アプリケーション共有状態。
pub struct AppState {
    /// シミュレーションイベントの broadcast 送信側。
    pub event_tx: broadcast::Sender<DarviumEvent>,
    /// 制御コマンド送信チャネル。
    pub cmd_tx: mpsc::UnboundedSender<SimCommand>,
    /// シミュレーション管理状態。
    pub(crate) sim: parking_lot::Mutex<SharedSimState>,
    /// 動的シミュレーションパラメータ（フロントエンドから変更可能）。
    pub sim_params: Arc<RwLock<SimulationParams>>,
}

// ============================================================
// セッションハンドラ
// ============================================================

/// WebSocket 接続を処理する。
///
/// 1. broadcast チャネルからイベントを受信し、クライアントに JSON として転送する（送信タスク）
/// 2. クライアントからの制御コマンドを受信し、シミュレーションマネージャに転送する（受信ループ）
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, ws_receiver) = socket.split();
    let mut rx = state.event_tx.subscribe();
    let cmd_tx = state.cmd_tx.clone();

    // 送信タスク: broadcast → WebSocket
    let send_handle = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = match serde_json::to_string(&event) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if ws_sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // 受信ループ: WebSocket → 制御コマンド
    let recv_handle = tokio::spawn(async move {
        let mut ws_stream = ws_receiver;
        while let Some(Ok(msg)) = ws_stream.next().await {
            if let Message::Text(text) = msg {
                let v: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                match v["command"].as_str() {
                    Some("start") => {
                        if let Some(config) = parse_config(&v["config"]) {
                            let _ = cmd_tx.send(SimCommand::Start(Box::new(config)));
                        }
                    }
                    Some("stop") => {
                        let _ = cmd_tx.send(SimCommand::Stop);
                    }
                    Some("resume") => {
                        let _ = cmd_tx.send(SimCommand::Resume);
                    }
                    Some("reset") => {
                        let _ = cmd_tx.send(SimCommand::Reset);
                    }
                    Some("update_param") => {
                        if let Some(config) = v.get("config") {
                            if let Some(md) = config.get("movement_distance").and_then(|v| v.as_f64()) {
                                let _ = cmd_tx.send(SimCommand::UpdateParam(md));
                            }
                            if let Some(cas) = config.get("chief_attraction_strength").and_then(|v| v.as_f64()) {
                                let _ = cmd_tx.send(SimCommand::UpdateChiefAttraction(cas));
                            }
                            if let Some(mad) = config.get("min_approach_distance").and_then(|v| v.as_f64()) {
                                let _ = cmd_tx.send(SimCommand::UpdateMinApproach(mad));
                            }
                            if let Some(tp) = config.get("target_population").and_then(|v| v.as_f64()) {
                                let _ = cmd_tx.send(SimCommand::UpdateTargetPop(tp as usize));
                            }
                            if let Some(rr) = config.get("pressure_ramp_range").and_then(|v| v.as_f64()) {
                                let mut params = state.sim_params.write().unwrap();
                                params.pressure_ramp_range = (rr as usize).clamp(1, 1000);
                            }
                            if let Some(ut) = config.get("pressure_ramp_up_ticks").and_then(|v| v.as_f64()) {
                                let mut params = state.sim_params.write().unwrap();
                                params.pressure_ramp_up_ticks = (ut as u64).clamp(0, 1000);
                            }
                            if let Some(dt) = config.get("pressure_ramp_down_ticks").and_then(|v| v.as_f64()) {
                                let mut params = state.sim_params.write().unwrap();
                                params.pressure_ramp_down_ticks = (dt as u64).clamp(0, 1000);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    tokio::select! {
        _ = send_handle => {},
        _ = recv_handle => {},
    }
}

/// WebSocket アップグレードハンドラ。
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

// ============================================================
// シミュレーション管理
// ============================================================

/// シミュレーションマネージャタスク。
///
/// 制御コマンドを受信し、シミュレーションの開始・停止・リセットを管理する。
async fn simulation_manager_task(
    mut cmd_rx: mpsc::UnboundedReceiver<SimCommand>,
    state: Arc<AppState>,
) {
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            SimCommand::Start(config) => {
                start_simulation(&state, *config);
            }
            SimCommand::Stop => {
                let sim = state.sim.lock();
                if let Some(cancel) = &sim.cancel {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            SimCommand::Resume => {
                let last = state.sim.lock().last_config.clone();
                if let Some(config) = last {
                    start_simulation(&state, config);
                }
            }
            SimCommand::UpdateParam(movement_distance) => {
                let mut params = state.sim_params.write().unwrap();
                let clamped = movement_distance
                    .clamp(0.001, params.min_approach_distance * 0.95);
                params.movement_distance = clamped;
            }
            SimCommand::UpdateChiefAttraction(strength) => {
                let clamped = strength.clamp(0.1, 10.0);
                let mut params = state.sim_params.write().unwrap();
                params.chief_attraction_strength = clamped;
            }
            SimCommand::UpdateMinApproach(distance) => {
                let clamped = distance.clamp(0.005, 0.50);
                let mut params = state.sim_params.write().unwrap();
                params.min_approach_distance = clamped;
            }
            SimCommand::UpdateTargetPop(tp) => {
                let clamped = tp.clamp(0, 10000);
                let mut params = state.sim_params.write().unwrap();
                params.target_population = clamped;
            }
            SimCommand::Reset => {
                let mut sim = state.sim.lock();
                if let Some(cancel) = &sim.cancel {
                    cancel.store(true, Ordering::Relaxed);
                }
                sim.cancel = None;
                sim.last_config = None;
            }
        }
    }
}

/// シミュレーションを開始する（既存のシミュレーションがあれば最初に停止する）。
fn start_simulation(state: &Arc<AppState>, config: ReciprocitySimulatorConfig) {
    // 既存のシミュレーションを停止
    {
        let sim = state.sim.lock();
        if let Some(cancel) = &sim.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let event_tx = state.event_tx.clone();

    {
        let mut sim = state.sim.lock();
        sim.cancel = Some(cancel.clone());
        sim.last_config = Some(config.clone());
    }

    let sim_params = state.sim_params.clone();
    tokio::spawn(async move {
        let event_ch = BroadcastEventChannel::new(event_tx);
        tokio::task::spawn_blocking(move || {
            let _ = run_evaluation_simulation_with_channel(&config, &event_ch, &cancel, Some(&sim_params));
        })
        .await
        .ok();
    });
}

// ============================================================
// 設定解析
// ============================================================

/// クライアントからの JSON 設定を ReciprocitySimulatorConfig に変換する。
///
/// 指定されなかったフィールドはデフォルト値を使用する。
fn parse_config(v: &serde_json::Value) -> Option<ReciprocitySimulatorConfig> {
    let obj = v.as_object()?;
    let mut cfg = ReciprocitySimulatorConfig {
        seed: None, // サーバーからのシミュレーションは非決定論的動作（seed 未指定時）
        ..Default::default()
    };

    if let Some(val) = obj.get("population_size").and_then(|v| v.as_u64()) {
        cfg.population_size = val as usize;
    }
    if let Some(val) = obj.get("child_ratio").and_then(|v| v.as_f64()) {
        cfg.child_ratio = val;
    }
    if let Some(val) = obj.get("mission_rate").and_then(|v| v.as_f64()) {
        cfg.mission_rate = val;
    }
    if let Some(val) = obj.get("max_ticks").and_then(|v| v.as_u64()) {
        cfg.max_ticks = val;
    }
    if let Some(val) = obj.get("gc_interval").and_then(|v| v.as_u64()) {
        cfg.gc_interval = val;
    }
    if let Some(val) = obj.get("seed").and_then(|v| v.as_u64()) {
        cfg.seed = Some(val);
    }
    if let Some(val) = obj.get("target_village_size").and_then(|v| v.as_f64()) {
        cfg.target_village_size = Some(val);
    }
    if let Some(val) = obj.get("village_recluster_interval").and_then(|v| v.as_u64()) {
        cfg.village_recluster_interval = val.max(1);
    }
    if let Some(val) = obj.get("use_gmr").and_then(|v| v.as_bool()) {
        cfg.use_gmr = val;
    }
    if let Some(val) = obj.get("skip_child_search").and_then(|v| v.as_bool()) {
        cfg.skip_child_search = val;
    }
    if let Some(val) = obj.get("reputation_recompute_interval").and_then(|v| v.as_u64()) {
        cfg.reputation_recompute_interval = val.max(1);
    }

    Some(cfg)
}

// ============================================================
// サーバー起動
// ============================================================

/// 観測サーバーを起動する。
///
/// # 引数
/// - `port`: リッスンポート
/// - `web_dir`: 静的ファイルを提供するディレクトリのパス
pub async fn run(port: u16, web_dir: String) {
    let (event_tx, _) = broadcast::channel::<DarviumEvent>(1024);
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<SimCommand>();

    let state = Arc::new(AppState {
        event_tx: event_tx.clone(),
        cmd_tx,
        sim: parking_lot::Mutex::new(SharedSimState {
            cancel: None,
            last_config: None,
        }),
        sim_params: Arc::new(RwLock::new(SimulationParams {
            movement_distance: crate::constants::MOVEMENT_DISTANCE,
            chief_attraction_strength: crate::constants::CHIEF_ATTRACTION_STRENGTH,
            min_approach_distance: crate::constants::MIN_APPROACH_DISTANCE,
            target_population: 0,
            pressure_ramp_range: 50,
            pressure_ramp_up_ticks: 10,
            pressure_ramp_down_ticks: 20,
        })),
    });

    // シミュレーションマネージャタスクを起動
    let mgr_state = state.clone();
    tokio::spawn(simulation_manager_task(cmd_rx, mgr_state));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(&web_dir))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("Darvium Observer サーバー起動: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("ポートのバインドに失敗しました");
    axum::serve(listener, app)
        .await
        .expect("サーバーの起動に失敗しました");
}

use actix_web::{web, App, HttpResponse, HttpServer, Responder, HttpRequest};
use actix_web_actors::ws;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::time::{Duration, Instant};
use futures_util::StreamExt;
use std::net::SocketAddr;
use actix::ActorContext;
use chrono::{DateTime, Utc};
use actix_cors::Cors;
use actix::Actor;
use actix::Context;
use actix::AsyncContext;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub payload_template: Option<serde_json::Value>,
    pub duration: u64,
    pub random_fields: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientStats {
    pub total_requests: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub avg_response_time: f64,
}

#[derive(Debug, Serialize)]
pub struct ClientInfo {
    pub id: String,
    pub connected_at: String,
    pub last_active: String,
    pub stats: ClientStats,
}

// 存储所有连接的客户端信息
type Clients = Arc<Mutex<HashMap<String, WebSocketConnection>>>;

// WebSocket连接处理
#[derive(Clone)]
struct WebSocketConnection {
    id: String,
    addr: SocketAddr,
    connected_at: DateTime<Utc>,
    last_active: Arc<Mutex<DateTime<Utc>>>,
    stats: Arc<Mutex<ClientStats>>,
    tx: mpsc::Sender<TaskConfig>,
    app_state: web::Data<AppState>,
}

struct AppState {
    clients: Arc<Mutex<HashMap<String, WebSocketConnection>>>,
}

impl WebSocketConnection {
    fn check_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let now = Utc::now();
        let last_active = self.last_active.lock().unwrap();
        if (now - *last_active).num_seconds() > 30 {  // 增加超时时间到30秒
            println!("客户端 {} 心跳超时，断开连接", self.id);
            let mut clients = self.app_state.clients.lock().unwrap();
            clients.remove(&self.id);
            ctx.stop();
        }
    }

    fn send_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        println!("服务端 -> 客户端 {}: 发送心跳", self.id);
        ctx.ping(b"");
    }
}

impl actix::Actor for WebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // 每5秒检查一次心跳
        ctx.run_interval(Duration::from_secs(5), |actor, ctx| {
            actor.check_heartbeat(ctx);
        });

        // 每15秒发送一次心跳
        ctx.run_interval(Duration::from_secs(15), |actor, ctx| {
            actor.send_heartbeat(ctx);
        });
    }
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                let mut last_active = self.last_active.lock().unwrap();
                *last_active = Utc::now();
                println!("服务端 <- 客户端 {}: 收到Ping", self.id);
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                let mut last_active = self.last_active.lock().unwrap();
                *last_active = Utc::now();
                println!("服务端 <- 客户端 {}: 收到Pong", self.id);
            }
            Ok(ws::Message::Text(text)) => {
                let mut last_active = self.last_active.lock().unwrap();
                *last_active = Utc::now();
                if let Ok(register_msg) = serde_json::from_str::<RegisterMessage>(&text) {
                    if register_msg.message_type == "register" {
                        println!("客户端注册: ID={}", register_msg.client_id);
                        // 更新客户端ID
                        let old_id = self.id.clone();
                        self.id = register_msg.client_id.clone();
                        // 更新客户端列表中的ID
                        let mut clients = self.app_state.clients.lock().unwrap();
                        if let Some(client) = clients.remove(&old_id) {
                            clients.insert(register_msg.client_id, client);
                        }
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                let mut last_active = self.last_active.lock().unwrap();
                *last_active = Utc::now();
            }
            Ok(ws::Message::Close(reason)) => {
                println!("客户端 {} 主动断开连接", self.id);
                let mut clients = self.app_state.clients.lock().unwrap();
                clients.remove(&self.id);
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                let mut clients = self.app_state.clients.lock().unwrap();
                clients.remove(&self.id);
                ctx.stop();
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterMessage {
    #[serde(rename = "type")]
    message_type: String,
    client_id: String,
}

// HTTP处理函数
async fn get_ws_address(clients: web::Data<Clients>) -> impl Responder {
    let clients = clients.lock().unwrap();
    let addresses: Vec<String> = clients
        .values()
        .map(|client| format!("ws://{}", client.addr))
        .collect();
    
    HttpResponse::Ok().json(addresses)
}

async fn register_client(
    client_id: web::Path<String>,
    addr: web::Data<SocketAddr>,
    clients: web::Data<Clients>,
) -> impl Responder {
    let mut clients = clients.lock().unwrap();
    let (tx, _) = mpsc::channel(100);
    let client_id = client_id.into_inner();
    clients.insert(client_id.clone(), WebSocketConnection {
        id: client_id,
        addr: *addr.get_ref(),
        connected_at: Utc::now(),
        last_active: Arc::new(Mutex::new(Utc::now())),
        stats: Arc::new(Mutex::new(ClientStats {
            total_requests: 0,
            success_count: 0,
            error_count: 0,
            avg_response_time: 0.0,
        })),
        tx,
        app_state: web::Data::new(AppState {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }),
    });
    HttpResponse::Ok().finish()
}

async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let client_id = generate_client_id();
    let (tx, _) = mpsc::channel(100);
    
    let ws_connection = WebSocketConnection {
        id: client_id.clone(),
        addr: req.peer_addr().unwrap(),
        connected_at: Utc::now(),
        last_active: Arc::new(Mutex::new(Utc::now())),
        stats: Arc::new(Mutex::new(ClientStats {
            total_requests: 0,
            success_count: 0,
            error_count: 0,
            avg_response_time: 0.0,
        })),
        tx: tx.clone(),
        app_state: data.clone(),
    };

    // 将新客户端添加到客户端列表中
    {
        let mut clients = data.clients.lock().unwrap();
        clients.insert(client_id.clone(), ws_connection.clone());
    }
    
    let resp = ws::start(ws_connection, &req, stream)?;

    Ok(resp)
}

async fn get_clients(data: web::Data<AppState>) -> HttpResponse {
    let clients = data.clients.lock().unwrap();
    let client_list: Vec<ClientInfo> = clients
        .values()
        .map(|client| ClientInfo {
            id: client.id.clone(),
            connected_at: client.connected_at.to_rfc3339(),
            last_active: client.last_active.lock().unwrap().to_rfc3339(),
            stats: client.stats.lock().unwrap().clone(),
        })
        .collect();
    
    HttpResponse::Ok().json(client_list)
}

async fn assign_task_all(
    task: web::Json<TaskConfig>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let mut clients = data.clients.lock().unwrap();
    let mut success_count = 0;
    let mut error_count = 0;
    let mut error_details = Vec::new();
    let mut disconnected_clients = Vec::new();

    for (client_id, client) in clients.iter() {
        match client.tx.send(task.0.clone()).await {
            Ok(_) => {
                success_count += 1;
                println!("成功发送任务到客户端: {}", client_id);
            }
            Err(e) => {
                error_count += 1;
                let error_msg = format!("客户端 {} 发送失败: {}", client_id, e);
                println!("{}", error_msg);
                error_details.push(error_msg);
                disconnected_clients.push(client_id.clone());
            }
        }
    }

    // 清理断开的客户端
    for client_id in disconnected_clients {
        clients.remove(&client_id);
        println!("清理断开的客户端: {}", client_id);
    }

    let response = if error_count > 0 {
        serde_json::json!({
            "message": format!("任务已下发到 {} 个客户端，成功: {}，失败: {}", 
                clients.len(), success_count, error_count),
            "errors": error_details
        })
    } else {
        serde_json::json!({
            "message": format!("任务已成功下发到 {} 个客户端", clients.len())
        })
    };

    HttpResponse::Ok().json(response)
}

fn generate_client_id() -> String {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    let random_suffix: String = (0..8)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect();
    format!("client-{}", random_suffix)
}

pub async fn run_server() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        clients: Arc::new(Mutex::new(HashMap::new())),
    });

    // 启动HTTP服务器
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600)
            )
            .app_data(app_state.clone())
            .route("/ws", web::get().to(ws_index))
            .route("/clients", web::get().to(get_clients))
            .route("/assign_all", web::post().to(assign_task_all))
    })
    .bind("0.0.0.0:8080")?
    .run();

    println!("服务器启动在 http://0.0.0.0:8080");
    server.await
} 
use std::collections::HashMap;
use std::net::{TcpListener, IpAddr};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use rand::{thread_rng, Rng};
use reqwest::Client;
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use serde_json::Value;
use std::error::Error;
use serde_json::json;
use tokio::time::sleep;
use std::sync::atomic::{AtomicU64, Ordering};
use local_ip_address::local_ip;
use crate::server::TaskConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientConfig {
    pub server: ServerConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub address: String,
    pub client_id: String,
    pub ws_port_range: [u16; 2],
    pub max_connections: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestConfig {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    payload_template: Option<Value>,
    duration: u64,
    random_fields: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_latency: u64,
    min_latency: u64,
    max_latency: u64,
    error_count: HashMap<String, u64>,
}

impl Default for ClientStats {
    fn default() -> Self {
        ClientStats {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_latency: 0,
            min_latency: u64::MAX,
            max_latency: 0,
            error_count: HashMap::new(),
        }
    }
}

impl ClientStats {
    fn update(&mut self, success: bool, latency: u64, error: Option<String>) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        self.total_latency += latency;
        self.min_latency = self.min_latency.min(latency);
        self.max_latency = self.max_latency.max(latency);
        if let Some(err) = error {
            *self.error_count.entry(err).or_insert(0) += 1;
        }
    }
}

async fn find_available_port(start: u16, end: u16) -> Option<u16> {
    for port in start..=end {
        if TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
            return Some(port);
        }
    }
    None
}

async fn generate_random_payload(template: &Value, random_fields: &[String]) -> Value {
    let mut rng = thread_rng();
    let mut payload = template.clone();
    
    for field in random_fields {
        if let Some(value) = payload.get_mut(field) {
            match value {
                Value::String(_) => {
                    *value = Value::String(generate_random_string(8));
                }
                Value::Number(n) if n.is_i64() => {
                    *value = Value::Number(serde_json::Number::from(rng.gen::<i64>()));
                }
                _ => {}
            }
        }
    }
    
    payload
}

fn generate_random_string(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

async fn run_load_test(config: TestConfig, stats: Arc<Mutex<ClientStats>>) {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let end_time = Instant::now() + Duration::from_secs(config.duration);
    
    while Instant::now() < end_time {
        let mut request = client.request(
            config.method.parse().unwrap(),
            &config.url
        );

        // 添加headers
        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        // 添加query参数
        request = request.query(&config.query_params);

        // 添加payload
        if let Some(template) = &config.payload_template {
            let payload = if let Some(random_fields) = &config.random_fields {
                generate_random_payload(template, random_fields).await
            } else {
                template.clone()
            };
            request = request.json(&payload);
        }

        let start_time = Instant::now();
        
        let result = request.send().await;
        
        let latency = start_time.elapsed().as_millis() as u64;
        let success = result.is_ok();
        let error = result.err().map(|e| e.to_string());
        
        let mut stats = stats.lock().await;
        stats.update(success, latency, error);
    }
}

async fn report_stats(
    write: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    stats: Arc<Mutex<ClientStats>>,
) {
    let stats = stats.lock().await;
    let report = serde_json::json!({
        "total_requests": stats.total_requests,
        "successful_requests": stats.successful_requests,
        "failed_requests": stats.failed_requests,
        "qps": stats.total_requests as f64 / stats.total_latency as f64 * 1000.0,
        "duration": stats.total_latency / 1000000,
        "min_latency": stats.min_latency / 1000,
        "max_latency": stats.max_latency / 1000,
        "error_count": stats.error_count.clone()
    });

    if let Err(e) = write.send(Message::Text(report.to_string())).await {
        eprintln!("发送统计信息失败: {}", e);
    }
}

async fn connect_to_server(server_addr: &str, client_id: &str) -> Result<(futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>, futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>, String), Box<dyn Error>> {
    let server_addr = server_addr.replace("http://", "ws://").replace("localhost", "127.0.0.1");
    let (ws_stream, _) = connect_async(format!("{}/ws", server_addr)).await?;
    let (mut write, read) = ws_stream.split();
    
    // 发送注册消息
    let register_msg = json!({
        "type": "register",
        "client_id": client_id
    });
    write.send(Message::Text(register_msg.to_string())).await?;
    
    Ok((write, read, server_addr))
}

pub async fn run_client(config: ClientConfig) -> Result<(), Box<dyn Error>> {
    println!("启动客户端模式...");
    
    let client_id = generate_client_id();
    println!("生成的客户端ID: {}", client_id);
    
    let stats = Arc::new(Mutex::new(ClientStats::default()));
    let mut retry_count = 0;
    let max_retries = 5;
    
    loop {
        match connect_to_server(&config.server.address, &client_id).await {
            Ok((write, mut read, server_addr)) => {
                println!("成功连接到服务器: {}", server_addr);
                retry_count = 0;
                
                // 将write流包装在Arc<Mutex>中以便共享
                let write = Arc::new(Mutex::new(write));
                
                // 创建心跳发送任务
                let write_clone = write.clone();
                let client_id_clone = client_id.clone();
                let heartbeat_task = tokio::spawn(async move {
                    loop {
                        println!("客户端 {} -> 服务端: 发送心跳", client_id_clone);
                        if write_clone.lock().await.send(Message::Ping(vec![])).await.is_err() {
                            println!("客户端 {} 心跳发送失败", client_id_clone);
                            break;
                        }
                        sleep(Duration::from_secs(15)).await;  // 每15秒发送一次心跳
                    }
                });
                
                // 处理来自服务器的消息
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Ping(data)) => {
                            println!("客户端 {} <- 服务端: 收到Ping", client_id);
                            if write.lock().await.send(Message::Pong(data)).await.is_err() {
                                println!("客户端 {} Pong响应发送失败", client_id);
                                break;
                            }
                        }
                        Ok(Message::Pong(_)) => {
                            println!("客户端 {} <- 服务端: 收到Pong", client_id);
                        }
                        Ok(Message::Text(text)) => {
                            if let Ok(task) = serde_json::from_str::<TaskConfig>(&text) {
                                println!("收到任务: {:?}", task);
                                let test_config = TestConfig {
                                    url: task.url,
                                    method: task.method,
                                    headers: task.headers,
                                    query_params: task.query_params,
                                    payload_template: task.payload_template,
                                    duration: task.duration,
                                    random_fields: Some(task.random_fields),
                                };
                                let stats = stats.clone();
                                tokio::spawn(async move {
                                    run_load_test(test_config, stats).await;
                                });
                            }
                        }
                        Ok(Message::Close(_)) => {
                            println!("服务器关闭连接");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket错误: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                
                // 取消心跳任务
                heartbeat_task.abort();
                println!("客户端 {} 连接已断开", client_id);
            }
            Err(e) => {
                eprintln!("客户端 {} 连接服务器失败: {}", client_id, e);
                retry_count += 1;
                if retry_count >= max_retries {
                    eprintln!("客户端 {} 达到最大重试次数，退出", client_id);
                    break;
                }
                println!("客户端 {} 等待5秒后重试...", client_id);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

pub async fn run(config: ClientConfig) -> Result<(), Box<dyn Error>> {
    match config.server.client_id.as_str() {
        "server" => {
            println!("启动服务器模式...");
            // 这里添加服务器模式的具体实现
            Ok(())
        }
        _ => run_client(config).await,
    }
}

fn generate_client_id() -> String {
    let ip = local_ip().unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let random_suffix = generate_random_string(8);
    format!("{}-{}", ip, random_suffix)
}
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rand::Rng;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use rayon::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    server: ServerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerConfig {
    address: String,
    client_id: Option<String>,
}

#[derive(Debug, Clone)]
struct Stats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    avg_latency: f64,
    min_latency: u64,
    max_latency: u64,
    error_count: HashMap<String, u64>,
    last_qps_update: Instant,
    requests_since_last_update: u64,
    current_qps: f64,
    last_response: Option<String>,
}

impl Stats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency: 0.0,
            min_latency: u64::MAX,
            max_latency: 0,
            error_count: HashMap::new(),
            last_qps_update: Instant::now(),
            requests_since_last_update: 0,
            current_qps: 0.0,
            last_response: None,
        }
    }

    fn update_qps(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_qps_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.current_qps = self.requests_since_last_update as f64 / elapsed;
            self.requests_since_last_update = 0;
            self.last_qps_update = now;
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskConfig {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    payload_template: Option<serde_json::Value>,
    duration: u64,
    random_fields: Vec<String>,
}

struct APIBurnerClient {
    config: Config,
    stats: Arc<Mutex<Stats>>,
    last_heartbeat: Instant,
    ws_sender: Arc<Mutex<Option<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>>>,
    heartbeat_timeout: Duration,
}

impl APIBurnerClient {
    fn new(config: Config) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(Stats::new())),
            last_heartbeat: Instant::now(),
            ws_sender: Arc::new(Mutex::new(None)),
            heartbeat_timeout: Duration::from_secs(30), // 30秒超时
        }
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut retry_count = 0;
        let max_retries = 5;
        let mut retry_delay = Duration::from_secs(5);

        loop {
            // 将 HTTP URL 转换为 WebSocket URL
            let ws_url = self.config.server.address.replace("http://", "ws://")
                .replace("https://", "wss://")
                + "/ws";
                
            println!("正在连接WebSocket: {} (尝试 {}/{})", ws_url, retry_count + 1, max_retries);
            
            // 清理旧的连接
            if let Some(mut sender) = self.ws_sender.lock().await.take() {
                if let Err(e) = sender.close().await {
                    println!("关闭旧连接时出错: {}", e);
                }
            }
            
            match connect_async(ws_url).await {
                Ok((ws_stream, _)) => {
                    println!("WebSocket连接成功");
                    let (write, mut read) = ws_stream.split();
                    *self.ws_sender.lock().await = Some(write);
                    retry_count = 0; // 重置重试计数
                    retry_delay = Duration::from_secs(5); // 重置重试延迟
                    self.last_heartbeat = Instant::now(); // 重置心跳时间

                    // 发送注册消息
                    let register_msg = serde_json::json!({
                        "type": "register"
                    });
                    if let Some(sender) = self.ws_sender.lock().await.as_mut() {
                        sender.send(Message::Text(register_msg.to_string())).await?;
                    }
                    println!("注册消息已发送");

                    // 等待注册成功消息
                    if let Some(Ok(Message::Text(text))) = read.next().await {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                            if data["type"] == "register_success" {
                                if let Some(client_id) = data["client_id"].as_str() {
                                    self.config.server.client_id = Some(client_id.to_string());
                                    println!("收到服务端分配的客户端ID: {}", client_id);
                                }
                            }
                        }
                    }

                    // 创建心跳检查任务
                    let last_heartbeat = self.last_heartbeat;
                    let heartbeat_timeout = self.heartbeat_timeout;
                    let heartbeat_check_handle = tokio::spawn(async move {
                        loop {
                            if last_heartbeat.elapsed() > heartbeat_timeout {
                                println!("服务端心跳超时，准备重新连接");
                                break;
                            }
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    });

                    // 创建心跳任务
                    let ws_sender = self.ws_sender.clone();
                    let client_id = self.config.server.client_id.clone().unwrap();
                    let heartbeat_handle = tokio::spawn(async move {
                        loop {
                            if let Some(sender) = ws_sender.lock().await.as_mut() {
                                let ping_msg = serde_json::json!({
                                    "type": "ping",
                                    "client_id": client_id
                                });
                                if let Err(e) = sender.send(Message::Text(ping_msg.to_string())).await {
                                    println!("发送心跳失败: {}", e);
                                } else {
                                    println!("发送心跳成功");
                                }
                            }
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    });

                    // 主事件循环
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                println!("收到消息: {}", text);
                                if let Err(e) = self.handle_message(&text).await {
                                    println!("处理消息时出错: {}", e);
                                }
                            }
                            Ok(Message::Ping(_)) => {
                                if let Some(sender) = self.ws_sender.lock().await.as_mut() {
                                    let pong_msg = serde_json::json!({
                                        "type": "pong",
                                        "client_id": self.config.server.client_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                                    });
                                    sender.send(Message::Text(pong_msg.to_string())).await?;
                                }
                                self.last_heartbeat = Instant::now();
                            }
                            Ok(Message::Close(_)) => {
                                println!("服务器正常关闭了连接");
                                break;
                            }
                            Err(e) => {
                                println!("WebSocket错误: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                    
                    // 取消心跳任务
                    heartbeat_handle.abort();
                    heartbeat_check_handle.abort();
                    
                    // 连接断开，等待一段时间再重试
                    println!("连接已断开，等待 {} 秒后重试...", retry_delay.as_secs());
                    tokio::time::sleep(retry_delay).await;
                }
                Err(e) => {
                    println!("连接失败: {}", e);
                    retry_count += 1;
                    
                    if retry_count >= max_retries {
                        println!("达到最大重试次数，退出程序");
                        return Err(Box::new(e));
                    }
                    
                    // 使用指数退避策略
                    retry_delay *= 2;
                    println!("{}秒后尝试重新连接...", retry_delay.as_secs());
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }

    async fn handle_message(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 首先尝试解析为通用消息格式
        #[derive(Debug, Serialize, Deserialize)]
        struct MessageWrapper {
            #[serde(rename = "type")]
            message_type: String,
        }

        if let Ok(wrapper) = serde_json::from_str::<MessageWrapper>(message) {
            match wrapper.message_type.as_str() {
                "ping" => {
                    println!("收到ping消息");
                    return Ok(());
                }
                "task" => {
                    // 如果是任务消息，解析为任务配置
                    let task: TaskConfig = serde_json::from_str(message)?;
                    println!("开始执行任务: {}", task.url);
                    self.execute_task(task).await?;
                    println!("任务执行完成");
                    return Ok(());
                }
                _ => {
                    println!("收到未知类型的消息: {}", message);
                    return Ok(());
                }
            }
        }

        // 如果不是通用消息格式，尝试直接解析为任务配置
        let task: TaskConfig = serde_json::from_str(message)?;
        println!("开始执行任务: {}", task.url);
        self.execute_task(task).await?;
        println!("任务执行完成");
        Ok(())
    }

    async fn execute_task(&mut self, task: TaskConfig) -> Result<(), Box<dyn std::error::Error>> {
        if task.url.is_empty() {
            println!("任务URL为空，跳过执行");
            return Ok(());
        }

        let url = if task.url.starts_with("http://") || task.url.starts_with("https://") {
            task.url.clone()
        } else {
            format!("http://{}", task.url)
        };

        let client = reqwest::Client::new();
        let start_time = Instant::now();
        let end_time = start_time + Duration::from_secs(task.duration);
        let stats = self.stats.clone();
        let ws_sender = self.ws_sender.clone();

        println!("任务将在 {} 秒内执行", task.duration);
        println!("目标URL: {}", url);

        // 创建定时发送统计信息到服务器的任务
        let stats_for_report = stats.clone();
        let report_handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                let stats = stats_for_report.lock().await;
                let stats_report = serde_json::json!({
                    "type": "stats",
                    "stats": {
                        "total_requests": stats.total_requests,
                        "success_count": stats.successful_requests,
                        "error_count": stats.failed_requests,
                        "avg_response_time": stats.avg_latency,
                        "current_qps": stats.current_qps
                    }
                });
                if let Some(sender) = ws_sender.lock().await.as_mut() {
                    if let Err(e) = sender.send(Message::Text(stats_report.to_string())).await {
                        println!("发送统计信息失败: {}", e);
                    }
                }
                drop(stats);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // 创建定时打印 QPS 和响应内容的任务
        let stats_for_print = stats.clone();
        let print_handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                let stats = stats_for_print.lock().await;
                println!("当前QPS: {:.2}", stats.current_qps);
                if let Some(response) = &stats.last_response {
                    println!("最新响应内容: {}", response);
                }
                drop(stats);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // 创建线程池
        let num_threads = num_cpus::get();
        let mut handles = Vec::new();

        for _ in 0..num_threads {
            let url = url.clone();
            let client = client.clone();
            let task = task.clone();
            let stats = stats.clone();

            let handle = tokio::spawn(async move {
                while Instant::now() < end_time {
                    let request_start = Instant::now();
                    
                    let mut request = client.request(
                        reqwest::Method::from_bytes(task.method.as_bytes()).unwrap(),
                        &url
                    );

                    // 添加请求头
                    for (key, value) in &task.headers {
                        request = request.header(key, value);
                    }

                    // 添加查询参数
                    request = request.query(&task.query_params);

                    // 如果有请求体，添加随机化后的请求体
                    if let Some(template) = &task.payload_template {
                        let payload = Self::randomize_payload(template, &task.random_fields);
                        request = request.json(&payload);
                    }

                    // 发送请求并更新统计信息
                    match request.send().await {
                        Ok(response) => {
                            let latency = request_start.elapsed().as_millis() as u64;
                            let mut stats = stats.lock().await;
                            stats.total_requests += 1;
                            stats.successful_requests += 1;
                            stats.requests_since_last_update += 1;
                            stats.min_latency = stats.min_latency.min(latency);
                            stats.max_latency = stats.max_latency.max(latency);
                            stats.avg_latency = (stats.avg_latency * (stats.successful_requests - 1) as f64
                                + latency as f64) / stats.successful_requests as f64;
                            stats.update_qps();
                            
                            // 获取响应内容
                            if let Ok(text) = response.text().await {
                                stats.last_response = Some(text);
                            }
                        }
                        Err(e) => {
                            let mut stats = stats.lock().await;
                            stats.total_requests += 1;
                            stats.failed_requests += 1;
                            stats.requests_since_last_update += 1;
                            *stats.error_count.entry(e.to_string()).or_insert(0) += 1;
                            stats.update_qps();
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.await?;
        }

        // 等待打印任务完成
        print_handle.await?;

        // 等待报告任务完成
        report_handle.await?;

        // 打印最终统计信息
        let stats = stats.lock().await;
        println!("\n任务执行完成，统计信息：");
        println!("总请求数: {}", stats.total_requests);
        println!("成功请求: {}", stats.successful_requests);
        println!("失败请求: {}", stats.failed_requests);
        println!("平均延迟: {:.2}ms", stats.avg_latency);
        println!("最小延迟: {}ms", stats.min_latency);
        println!("最大延迟: {}ms", stats.max_latency);
        println!("当前QPS: {:.2}", stats.current_qps);
        if !stats.error_count.is_empty() {
            println!("错误统计:");
            for (error, count) in &stats.error_count {
                println!("  {}: {}", error, count);
            }
        }

        Ok(())
    }

    fn randomize_payload(template: &serde_json::Value, random_fields: &[String]) -> serde_json::Value {
        let mut rng = rand::thread_rng();
        let mut result = template.clone();

        for field in random_fields {
            let mut value = &mut result;
            let segments: Vec<&str> = field.split('.').collect();
            
            // 遍历除最后一个段以外的所有段
            for &segment in segments.iter().take(segments.len() - 1) {
                match value {
                    serde_json::Value::Object(ref mut map) => {
                        if !map.contains_key(segment) {
                            map.insert(segment.to_string(), serde_json::Value::Object(serde_json::Map::new()));
                        }
                        value = map.get_mut(segment).unwrap();
                    }
                    _ => break,
                }
            }

            // 处理最后一个段
            if let Some(&last_segment) = segments.last() {
                if let serde_json::Value::Object(ref mut map) = value {
                    let random_value = match map.get(last_segment) {
                        Some(original) => match original {
                            serde_json::Value::String(_) => {
                                let length = rng.gen_range(5..20);
                                let random_string: String = (0..length)
                                    .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                                    .collect();
                                serde_json::Value::String(random_string)
                            },
                            serde_json::Value::Number(n) => {
                                if n.is_i64() {
                                    let random_int = rng.gen_range(1..1000);
                                    serde_json::Value::Number(serde_json::Number::from(random_int))
                                } else {
                                    let random_float = rng.gen_range(0.0..100.0);
                                    serde_json::json!(random_float)
                                }
                            },
                            serde_json::Value::Bool(_) => {
                                serde_json::Value::Bool(rng.gen_bool(0.5))
                            },
                            _ => original.clone(),
                        },
                        None => {
                            let length = rng.gen_range(5..20);
                            let random_string: String = (0..length)
                                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                                .collect();
                            serde_json::Value::String(random_string)
                        }
                    };
                    map.insert(last_segment.to_string(), random_value);
                }
            }
        }
        
        result
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 读取配置文件
    let config_path = Path::new("config.toml");
    let config_content = fs::read_to_string(config_path)
        .expect("无法读取配置文件");
    
    let config: Config = toml::from_str(&config_content)
        .expect("无法解析配置文件");

    println!("正在连接到服务器: {}", config.server.address);
    
    let mut client = APIBurnerClient::new(config);
    client.connect().await?;

    Ok(())
} 
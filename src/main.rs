use clap::Parser;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 并发进程数
    #[arg(short, long, default_value_t = 4)]
    processes: usize,

    /// 每个进程的线程数
    #[arg(short, long, default_value_t = 10)]
    threads: usize,

    /// 测试持续时间（秒）
    #[arg(short, long, default_value_t = 60)]
    duration: u64,

    /// 目标URL
    #[arg(short, long, default_value = "http://10.191.31.150/sso/api/login/")]
    url: String,

    /// 是否显示请求和响应内容
    #[arg(short, long, default_value_t = false)]
    show_response: bool,

    /// QPS统计窗口大小（秒）
    #[arg(short, long, default_value_t = 1)]
    qps_window: u64,
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

struct QpsStats {
    window_size: Duration,
    last_count: AtomicU64,
    last_time: Instant,
}

impl QpsStats {
    fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            last_count: AtomicU64::new(0),
            last_time: Instant::now(),
        }
    }

    fn update(&mut self, total_count: u64) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time);
        
        if elapsed >= self.window_size {
            let count_diff = total_count - self.last_count.load(Ordering::Relaxed);
            let qps = count_diff as f64 / elapsed.as_secs_f64();
            println!("当前QPS: {:.2}", qps);
            
            self.last_count.store(total_count, Ordering::Relaxed);
            self.last_time = now;
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let counter = Arc::new(AtomicU64::new(0));
    
    let client = Arc::new(Client::builder()
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(args.threads * args.processes) // 设置每个主机的最大空闲连接数
        .pool_idle_timeout(Duration::from_secs(60)) // 空闲连接保持60秒
        .tcp_keepalive(Duration::from_secs(30)) // TCP保持活动状态
        .http2_keep_alive_interval(Duration::from_secs(20)) // HTTP/2保持活动间隔
        .http2_keep_alive_timeout(Duration::from_secs(10)) // HTTP/2保持活动超时
        .build()
        .unwrap());

    println!("开始压测...");
    println!("进程数: {}", args.processes);
    println!("每进程线程数: {}", args.threads);
    println!("持续时间: {}秒", args.duration);
    println!("显示响应: {}", if args.show_response { "是" } else { "否" });
    println!("QPS统计窗口: {}秒", args.qps_window);

    let start_time = std::time::Instant::now();
    let mut qps_stats = QpsStats::new(Duration::from_secs(args.qps_window));
    
    // 创建多个进程
    let handles: Vec<_> = (0..args.processes)
        .map(|_| {
            let counter = Arc::clone(&counter);
            let client = Arc::clone(&client);
            let url = args.url.clone();
            let show_response = args.show_response;
            
            std::thread::spawn(move || {
                // 每个进程创建多个线程
                (0..args.threads).into_par_iter().for_each(|_| {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        while start_time.elapsed() < Duration::from_secs(args.duration) {
                            let username = generate_random_string(8);
                            let password = generate_random_string(8);

                            let payload = serde_json::json!({
                                "userName": username,
                                "password": password
                            });

                            if show_response {
                                println!("发送请求: {}", serde_json::to_string_pretty(&payload).unwrap());
                            }

                            let response = client
                                .post(&url)
                                .json(&payload)
                                .send()
                                .await;

                            if show_response {
                                match response {
                                    Ok(resp) => {
                                        match resp.text().await {
                                            Ok(text) => println!("响应内容: {}", text),
                                            Err(e) => println!("读取响应失败: {}", e),
                                        }
                                    }
                                    Err(e) => println!("请求失败: {}", e),
                                }
                            }

                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                });
            })
        })
        .collect();

    // 主线程负责统计QPS
    while start_time.elapsed() < Duration::from_secs(args.duration) {
        let total_count = counter.load(Ordering::Relaxed);
        qps_stats.update(total_count);
        std::thread::sleep(Duration::from_millis(100));
    }

    // 等待所有进程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let total_requests = counter.load(Ordering::Relaxed);
    let duration = start_time.elapsed().as_secs_f64();
    let rps = total_requests as f64 / duration;

    println!("\n压测结果:");
    println!("总请求数: {}", total_requests);
    println!("总耗时: {:.2}秒", duration);
    println!("平均RPS: {:.2}", rps);
}

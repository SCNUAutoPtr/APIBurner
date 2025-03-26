use clap::Parser;
use std::fs;
use toml;

mod server;
mod client;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 运行模式：server 或 client
    #[arg(short, long, default_value = "client")]
    mode: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args = Args::parse();

    // 读取配置文件
    let config_str = fs::read_to_string("config.toml")?;
    let config: toml::Value = toml::from_str(&config_str)?;

    match args.mode.as_str() {
        "server" => {
            println!("启动服务器模式...");
            server::run_server().await?;
        }
        "client" => {
            println!("启动客户端模式...");
            let client_config = config.try_into()?;
            client::run_client(client_config).await?;
        }
        _ => {
            println!("未知模式: {}", args.mode);
            std::process::exit(1);
        }
    }

    Ok(())
} 
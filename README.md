# APIBurner

APIBurner 是一个分布式 API 压测工具，支持多客户端协同压测，实时统计和报告。

## 功能特点

- 分布式压测：支持多客户端协同压测
- 实时统计：实时收集和展示压测数据
- 灵活配置：支持自定义请求参数、随机数据生成
- WebSocket 通信：使用 WebSocket 实现服务器和客户端之间的实时通信
- 支持多种 HTTP 方法：GET、POST、PUT、DELETE 等
- 支持自定义请求头和查询参数
- 支持 JSON 格式的请求体
- 支持随机字段生成

## 安装

1. 确保已安装 Rust 开发环境（Rust 1.70 或更高版本）
2. 克隆项目：
   ```bash
   git clone https://github.com/yourusername/APIBurner.git
   cd APIBurner
   ```
3. 编译项目：
   ```bash
   cargo build --release
   ```

## 配置

在项目根目录创建 `config.toml` 文件：

```toml
[server]
# 中心服务器地址
address = "http://localhost:8080"
# 客户端ID（可以是机器名或其他唯一标识）
client_id = "client-1"
# 本机WebSocket端口范围
ws_port_range = [8081, 9000]
# 最大并发连接数
max_connections = 1000
```

## 使用方法

### 启动服务器

```bash
cargo run -- --mode server
# 或使用短选项
cargo run -- -m server
```

服务器将在 8080 端口启动，等待客户端连接。

### 启动客户端

```bash
cargo run -- --mode client
# 或使用短选项
cargo run -- -m client
```

客户端将：
1. 连接到中心服务器
2. 注册自己的 WebSocket 端口
3. 等待服务器下发压测任务
4. 执行压测并报告结果

### 命令行参数

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| 运行模式 | -m | --mode | client | 运行模式：server 或 client |

### 下发压测任务

使用 curl 或 Postman 向服务器发送 POST 请求：

```bash
curl -X POST http://localhost:8080/assign/client-1 \
-H "Content-Type: application/json" \
-d '{
  "url": "http://example.com/api",
  "method": "POST",
  "headers": {
    "Content-Type": "application/json"
  },
  "query_params": {},
  "payload_template": {
    "name": "test",
    "age": 25
  },
  "duration": 60,
  "random_fields": ["name"]
}'
```

任务配置说明：
- `url`: 目标 API 地址
- `method`: HTTP 方法（GET、POST、PUT、DELETE）
- `headers`: 请求头
- `query_params`: URL 查询参数
- `payload_template`: 请求体模板
- `duration`: 测试持续时间（秒）
- `random_fields`: 需要随机化的字段列表

## 统计信息

客户端会定期向服务器报告以下统计信息：
- 总请求数
- 成功请求数
- 失败请求数
- 平均响应时间
- 最小响应时间
- 最大响应时间
- 错误类型统计

## 注意事项

1. 确保服务器和客户端之间的网络连接正常
2. 确保配置的端口未被其他程序占用
3. 压测时注意控制并发数，避免对目标系统造成过大压力
4. 建议在测试环境中使用，避免影响生产系统

## 开发计划

- [ ] 添加 Web 界面，可视化展示压测结果
- [ ] 支持更多数据格式（XML、Form 等）
- [ ] 添加压测任务模板功能
- [ ] 支持自定义压测脚本
- [ ] 添加压测报告导出功能

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License 
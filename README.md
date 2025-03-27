# APIBurner

APIBurner 是一个分布式 API 压测工具，支持多客户端协同压测，实时统计和报告。
Powered By Manybits x AutoPtr
©2025
Author:uaih3k9x

###碎碎念
因为我是懒狗所以服务端是NodeJS写的
保证客户端性能，客户端是rust写的
实现了简单的GetPost，请求头配置，查询参数配置，Body配置，随机字段

## 功能特点

- 分布式压测：支持多客户端协同压测
- 实时统计：实时收集和展示压测数据
- 灵活配置：支持自定义请求参数、随机数据生成
- WebSocket 通信：使用 WebSocket 实现服务器和客户端之间的实时通信
- 支持多种 HTTP 方法：GET、POST、PUT、DELETE 等
- 支持自定义请求头和查询参数
- 支持 JSON 格式的请求体
- 支持随机字段生成
- 自动重连机制
- 心跳检测
- 实时错误统计和报告

## 系统要求

- Rust 1.70 或更高版本
- Node.js 14.0 或更高版本（用于 Web 界面）
- 现代浏览器（Chrome、Firefox、Safari 等）

## 安装

1. 确保已安装 Rust 开发环境
2. 克隆项目：
   ```bash
   git clone https://github.com/yourusername/APIBurner.git
   cd APIBurner
   ```
3. 编译项目：
   ```bash
   cargo build --release
   ```

## 快速开始

### 1. 启动服务器
```bash
npm install
node server.js
```

### 2. 启动客户端
```bash
cargo run -- --mode client
```

### 2.1 启动网页服务器
```bash
cd web
npm install
npm run dev
```
### 3. 访问 Web 界面
打开浏览器访问 `http://localhost:5173` 查看压测控制面板。

## 配置

### 服务器配置 (config.toml)
```toml
[server]
address = "http://localhost:8080"
client_id = "client-1"
ws_port_range = [8081, 9000]
max_connections = 1000
```

### 客户端配置
```javascript
[server]
# 中心服务器地址
address = "http://127.0.0.1:8080"
# 可选：客户端ID，如不指定将自动
```

## API 文档

详细的 API 文档请参考：
- [服务器 API 文档](server.md)
- [客户端 API 文档](client.md)

## 压测任务配置

### 基本任务配置
```json
{
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
}
```

### 高级配置选项
- `concurrent_requests`: 并发请求数
- `request_timeout`: 请求超时时间（毫秒）
- `retry_count`: 失败重试次数
- `custom_scripts`: 自定义压测脚本

## 监控指标

### 实时统计
- 总请求数
- 成功请求数
- 失败请求数
- 平均响应时间
- 最小响应时间
- 最大响应时间
- 错误类型统计
- QPS（每秒查询数）
- 并发连接数

### 错误分析
- HTTP 状态码分布
- 网络错误统计
- 超时统计
- 自定义错误分类

## 最佳实践

1. 压测前准备
   - 确保目标系统处于稳定状态
   - 设置合理的并发数和持续时间
   - 准备充足的测试数据

2. 监控建议
   - 实时观察系统资源使用情况
   - 设置关键指标告警阈值
   - 保存压测日志以便分析

3. 性能优化
   - 合理设置连接池大小
   - 优化请求参数和头信息
   - 使用适当的超时设置

## 常见问题

1. 连接问题
   - 检查网络连接
   - 验证端口配置
   - 确认防火墙设置

2. 性能问题
   - 调整并发数
   - 优化请求参数
   - 检查系统资源

3. 数据问题
   - 验证请求格式
   - 检查随机数据生成
   - 确认响应处理

## 开发计划

- [x] 基础压测功能
- [x] WebSocket 通信
- [x] 实时统计
- [ ] Web 界面优化
- [ ] 更多数据格式支持
- [ ] 压测任务模板
- [ ] 自定义压测脚本
- [ ] 压测报告导出
- [ ] 分布式集群支持
- [ ] 性能分析工具

## 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 联系方式

- 项目主页：[GitHub](https://github.com/yourusername/APIBurner)
- 问题反馈：[Issues](https://github.com/yourusername/APIBurner/issues)
- 邮件联系：your.email@example.com 
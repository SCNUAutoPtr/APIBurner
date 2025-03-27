我来为您编写一个详细的 API 文档：

# APIBurner API 文档

## 基础信息
- 基础URL: `http://localhost:8080`
- 所有 WebSocket 连接使用 `ws://` 协议
- 所有响应均为 JSON 格式

## WebSocket 连接

### 连接端点
```
GET /ws
```

### WebSocket 消息格式

#### 1. 客户端注册消息
```json
{
    "type": "register",
    "client_id": "string"
}
```

#### 2. 任务配置消息
```json
{
    "url": "string",
    "method": "string",
    "headers": {
        "key": "value"
    },
    "query_params": {
        "key": "value"
    },
    "payload_template": {
        // 可选的 JSON 对象
    },
    "duration": number,
    "random_fields": ["string"]
}
```

## HTTP API

### 1. 获取客户端列表
```
GET /clients
```

响应示例：
```json
[
    {
        "id": "string",
        "connected_at": "ISO8601时间戳",
        "last_active": "ISO8601时间戳",
        "stats": {
            "total_requests": number,
            "success_count": number,
            "error_count": number,
            "avg_response_time": number
        }
    }
]
```

### 2. 下发任务到所有客户端
```
POST /assign_all
```

请求体：
```json
{
    "url": "string",
    "method": "string",
    "headers": {
        "key": "value"
    },
    "query_params": {
        "key": "value"
    },
    "payload_template": {
        // 可选的 JSON 对象
    },
    "duration": number,
    "random_fields": ["string"]
}
```

功能细节：
1.维护一个表，这个表会存储所有客户端的信息

响应示例：
```json
{
    "message": "任务已成功下发到 X 个客户端"
}
```

或错误响应：
```json
{
    "message": "任务已下发到 X 个客户端，成功: Y，失败: Z",
    "errors": [
        "错误详情1",
        "错误详情2"
    ]
}
```

## 心跳机制
- 服务器每15秒发送一次心跳（Ping）
- 客户端收到 Ping 后必须回复 Pong
- 如果30秒内没有收到客户端的心跳响应，服务器会断开连接

## 错误处理
- 所有 WebSocket 连接错误都会导致连接断开
- 任务发送失败会自动清理断开的客户端
- 客户端断开连接会自动从服务器列表中移除

## 注意事项
1. 所有时间戳使用 ISO8601 格式
2. 任务持续时间单位为秒
3. 响应时间单位为毫秒
4. 支持所有标准 HTTP 方法
5. 支持自定义请求头和查询参数
6. 支持 JSON 格式的请求体
7. 支持随机字段生成

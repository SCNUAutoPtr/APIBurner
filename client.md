# APIBurner 客户端文档

## 安装
```bash
npm install apiburner-client
```

## 基本用法

```javascript
const APIBurnerClient = require('apiburner-client');

const config = {
    server: {
        address: 'http://localhost:8080',
        client_id: 'custom-client-id'  // 可选，如不提供将自动生成
    }
};

const client = new APIBurnerClient(config);
client.connect();
```

## 配置选项

### ServerConfig
| 参数 | 类型 | 描述 | 必填 |
|------|------|------|------|
| address | string | 服务器地址 | 是 |
| client_id | string | 客户端ID | 否 |

## 功能特性

1. 自动重连机制
2. 心跳检测
3. 自动统计请求数据
   - 总请求数
   - 成功请求数
   - 失败请求数
   - 平均响应时间
   - 最小响应时间
   - 最大响应时间
   - 错误统计

4. 支持随机数据生成
   - 字符串随机化
   - 数字随机化

5. 任务执行能力
   - 支持 HTTP 所有方法
   - 支持自定义请求头
   - 支持查询参数
   - 支持请求体模板
   - 支持指定字段随机化

## 事件处理

客户端会自动处理以下 WebSocket 事件：
- 连接建立
- 心跳检测
- 任务接收
- 统计报告
- 错误处理
- 连接断开

## 统计指标

每个任务执行完成后，客户端会向服务器报告以下统计数据：

```typescript
interface Stats {
    totalRequests: number;      // 总请求数
    successfulRequests: number; // 成功请求数
    failedRequests: number;     // 失败请求数
    avgLatency: number;         // 平均延迟（毫秒）
    minLatency: number;         // 最小延迟（毫秒）
    maxLatency: number;         // 最大延迟（毫秒）
    errorCount: {              // 错误统计
        [errorType: string]: number;
    };
}
```

## 注意事项

1. 确保服务器地址正确配置
2. 网络异常时客户端会自动重连
3. 每15秒发送一次心跳包
4. 任务执行期间会实时收集统计数据
5. 支持并发任务执行
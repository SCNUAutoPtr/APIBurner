const express = require('express');
const WebSocket = require('ws');
const http = require('http');
const { v4: uuidv4 } = require('uuid');
const cors = require('cors');

// 创建 Express 应用和 HTTP 服务器
const app = express();
const server = http.createServer(app);
const wss = new WebSocket.Server({ server });

// 配置 CORS
app.use(cors({
    origin: '*', // 允许所有来源访问
    methods: ['GET', 'POST'], // 允许的 HTTP 方法
    allowedHeaders: ['Content-Type'] // 允许的请求头
}));

app.use(express.json());

// 存储客户端信息
const clients = new Map();

// 心跳检查
const HEARTBEAT_INTERVAL = 5000; // 15秒发送一次心跳
const HEARTBEAT_TIMEOUT = 30000;  // 30秒没有响应就断开连接

setInterval(() => {
    const now = Date.now();
    wss.clients.forEach((ws) => {
        if (ws.isAlive === false) {
            console.log('客户端心跳超时，断开连接');
            return ws.terminate();
        }
        // 获取该WebSocket连接对应的客户端ID
        const clientId = Array.from(clients.entries()).find(([_, client]) => client.ws === ws)?.[0];
        if (clientId) {
            ws.send(JSON.stringify({ 
                type: 'ping',
                client_id: clientId
            }));
        }
    });
}, HEARTBEAT_INTERVAL);

// WebSocket 连接处理
wss.on('connection', (ws) => {
    let clientId = null;
    let heartbeatTimeout = null;
    
    // 设置连接为活跃状态
    ws.isAlive = true;

    // 处理接收到的消息
    ws.on('message', (message) => {
        try {
            const data = JSON.parse(message);
            
            if (data.type === 'register') {
                clientId = uuidv4(); // 服务端生成clientId
                console.log(`客户端已注册`,data);
                clients.set(clientId, {
                    ws,
                    id: clientId,
                    connected_at: new Date().toISOString(),
                    last_active: new Date().toISOString(),
                    stats: {
                        total_requests: 0,
                        success_count: 0,
                        error_count: 0,
                        avg_response_time: 0,
                        current_qps: 0
                    }
                });
                // 发送注册成功消息，包含生成的clientId
                ws.send(JSON.stringify({
                    type: 'register_success',
                    client_id: clientId
                }));
                console.log(`客户端 ${clientId} 已注册`);
            } else if (data.type === 'pong') {
                console.log(`收到客户端 ${data.client_id} 的pong消息`);
                console.log(`data:${data}`);
                if (data.client_id && data.client_id === clientId) {
                    const client = clients.get(clientId);
                    if (client) {
                        client.last_active = new Date().toISOString();
                        ws.isAlive = true; // 更新活跃状态
                        // 清除之前的超时定时器
                        if (heartbeatTimeout) {
                            clearTimeout(heartbeatTimeout);
                        }
                        // 设置新的超时定时器
                    }
                } else {
                    console.warn(`收到不匹配的pong消息，期望的clientId: ${clientId}, 收到的clientId: ${data.client_id}`);
                }
            } else if (data.type === 'stats') {
                if (clientId) {
                    const client = clients.get(clientId);
                    if (client) {
                        client.stats = data.stats;
                        client.last_active = new Date().toISOString();
                        console.log(`客户端 ${clientId} 统计信息更新:`, data.stats);
                    }
                }
            } else if (data.type === 'ping') {
                if (data.client_id && data.client_id === clientId) {
                    console.log(`收到来自客户端 ${data.client_id} 的ping消息`);
                    //清除旧的定时器
                    if (heartbeatTimeout) {
                        clearTimeout(heartbeatTimeout);
                    }
                    console.log(`data:${JSON.stringify(data)}`);
                    // 设置新的超时定时器
                    heartbeatTimeout = setTimeout(() => {
                        console.log(`客户端 ${clientId} 心跳超时，断开连接`);
                        ws.terminate();
                    }, HEARTBEAT_TIMEOUT);
                    ws.send(JSON.stringify({ 
                        type: 'pong',
                        client_id: clientId  // 使用注册时的clientId
                    }));
                    const client = clients.get(clientId);
                    if (client) {
                        client.last_active = new Date().toISOString();
                    }
                } else {
                    console.warn(`收到不匹配的ping消息，期望的clientId: ${clientId}, 收到的clientId: ${data.client_id}`);
                }
            }
        } catch (err) {
            console.error('消息处理错误:', err);
        }
    });

    // 处理连接关闭
    ws.on('close', () => {
        if (clientId) {
            clients.delete(clientId);
            console.log(`客户端 ${clientId} 已断开连接`);
        }
        if (heartbeatTimeout) {
            clearTimeout(heartbeatTimeout);
        }
    });

    // 处理错误
    ws.on('error', (error) => {
        console.error(`WebSocket错误 (客户端 ${clientId || '未注册'}):`, error);
        if (clientId) {
            clients.delete(clientId);
        }
        if (heartbeatTimeout) {
            clearTimeout(heartbeatTimeout);
        }
    });
});

// HTTP API 路由
// 获取客户端列表
app.get('/clients', (req, res) => {
    console.log('获取客户端列表');
    const clientList = Array.from(clients.values()).map(({ ws, ...client }) => client);
    res.json(clientList);
});
app.get('/ping', (req, res) => {
    res.json({ message: 'pong' });
});


// 下发任务到所有客户端
app.post('/assign_all', (req, res) => {
    const task = req.body;
    const results = {
        success: 0,
        failed: 0,
        errors: []
    };

    clients.forEach((client, clientId) => {
        try {
            client.ws.send(JSON.stringify({
                type: 'task',
                ...task
            }));
            results.success++;
        } catch (err) {
            results.failed++;
            results.errors.push(`客户端 ${clientId} 发送失败: ${err.message}`);
        }
    });

    res.json({
        message: `任务已下发到 ${clients.size} 个客户端，成功: ${results.success}，失败: ${results.failed}`,
        errors: results.errors
    });
});

// 启动服务器
const PORT = process.env.PORT || 8080;
server.listen(PORT, () => {
    console.log(`服务器运行在 http://localhost:${PORT}`);
}); 
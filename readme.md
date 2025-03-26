# APIBurner

APIBurner 是一个高性能API压力测试工具，用Rust编写，支持高并发、多进程的API负载测试。

## 特性

- 多进程+多线程并发模型
- 实时QPS监控
- 可配置测试持续时间和并发级别
- 支持HTTP/HTTPS接口测试
- 自动化预检测试
- 可定制请求参数

## 安装

确保您已安装Rust和Cargo:

```bash

# 克隆仓库

gitclonehttps://github.com/yourusername/APIBurner.git

cdAPIBurner


# 构建项目

cargobuild--release

```

## 使用方法

基本用法:

```bash

# 使用默认配置

cargorun----i-agree


# 或使用构建后的可执行文件

./target/release/apiburner--i-agree

```

参数说明:

```

OPTIONS:

    -p, --processes <PROCESSES>            并发进程数 [默认: 4]

    -t, --threads <THREADS>                每个进程的线程数 [默认: 10]

    -d, --duration <DURATION>              测试持续时间（秒）[默认: 60]

    -u, --url <URL>                        目标URL [默认: "http://example.com/api"]

    -s, --show-response                    是否显示请求和响应内容 [默认: false]

    -q, --qps-window <QPS_WINDOW>          QPS统计窗口大小（秒）[默认: 1]

    -i, --request-interval <INTERVAL>      请求间隔毫秒数 [默认: 0]

    --skip-pretest                         跳过预检测试 [默认: false]

    --i-agree                              确认已阅读免责声明 [必需]

    -h, --help                             打印帮助信息

    -V, --version                          打印版本信息

```

示例:

```bash

# 使用100个线程，持续30秒进行测试

cargorun---p10-t10-d30--i-agree-u"http://your-api.com/endpoint"


# 测试并显示详细响应

cargorun---s--i-agree

```

## 指标监控

APIBurner 会在测试期间显示实时QPS，并在测试结束后提供以下统计信息:

- 总请求数
- 测试总耗时
- 平均每秒请求数 (RPS)

## 免责声明

本工具仅用于授权的性能测试和系统评估。使用本工具进行未经授权的测试可能违反相关法律法规。使用者必须确保已获得测试目标系统的授权，并对工具使用后果负全责。

## 许可证

MIT

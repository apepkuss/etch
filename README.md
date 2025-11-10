# Echo System - 智能音箱管理系统

Echo System 是一个完整的智能音箱管理平台，集成了语音识别、自然语言处理和音频流处理功能。

## 快速开始

### 1. 启动系统

```bash
./scripts/start.sh
```

或使用 Makefile：

```bash
make deploy
```

### 2. 验证部署

```bash
./scripts/verify-deployment.sh
```

或使用 Makefile：

```bash
make verify
```

### 3. 访问系统

- **Web管理界面**: <http://localhost:10034>
  - 用户名: `admin`
  - 密码: `admin123`

- **API Gateway**: <http://localhost:10033>
- **API健康检查**: <http://localhost:10033/health>

## 文档

详细的部署和配置说明请参考：

- [部署指南](DEPLOYMENT.md) - 完整的部署文档
- [系统设计](docs/SYSTEM_DESIGN.md) - 系统架构设计
- [端口配置](docs/PORT_CONFIGURATION.md) - 端口映射说明

## 管理命令

所有管理脚本都位于 `scripts/` 目录：

- `scripts/start.sh` - 启动所有服务
- `scripts/verify-deployment.sh` - 验证部署

推荐使用 Makefile 命令：

```bash
make help           # 查看所有可用命令
make deploy         # 完整部署
make up             # 启动服务
make down           # 停止服务
make logs           # 查看日志
make test-api       # 测试 API
```

## 系统架构

### 核心服务

| 服务 | 端口 | 描述 |
|------|------|------|
| Web Management | 10034 | React管理界面 |
| API Gateway | 10033 | HTTP API服务 |
| Bridge | 10031, 10032 | WebSocket/UDP桥接 |
| PostgreSQL | 10035 | 主数据库 |
| Redis | 10036 | 缓存服务 |
| MQTT Broker | 10039 | 消息代理 |

### 外部服务

- **EchoKit Server**: <https://indie.echokit.dev> (外部AI服务)

## 开发

```bash
# 开发模式启动 API Gateway
make dev-api

# 开发模式启动 Bridge
make dev-bridge

# 开发模式启动 Web 界面
make dev-web
```

## 支持

如遇到问题，请：

1. 检查日志: `make logs`
2. 验证部署: `./scripts/verify-deployment.sh`
3. 查看[部署指南](DEPLOYMENT.md)
4. 提交 Issue

## 许可证

[待添加]

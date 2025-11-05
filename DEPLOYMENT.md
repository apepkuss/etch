# Echo System Docker Compose 部署指南

## 概述

Echo System 提供了完整的 Docker Compose 部署方案，包含所有必要的组件和服务。

## 系统架构

### 核心服务

| 服务 | 主机端口 | 容器端口 | 协议 | 用途 |
|------|----------|----------|------|------|
| **API Gateway** | 10033 | 8080 | HTTP | HTTP API服务 |
| **Web Management** | 10034 | 5174 | HTTP | React管理界面 |
| **Bridge** | 10031, 10032 | WebSocket/UDP桥接服务 |
| **PostgreSQL** | 10035 | 主数据库 |
| **Redis** | 10036 | 缓存服务 |
| **MQTT Broker** | 10039, 10040 | 消息代理 |
| **EchoKit Server** | - | 外部AI服务 (https://indie.echokit.dev) |

### 管理工具

| 服务 | 端口 | 描述 |
|------|------|------|
| **pgAdmin** | 10037 | PostgreSQL管理界面 |
| **Redis Commander** | 10038 | Redis管理界面 |

## 快速开始

### 1. 环境准备

确保系统已安装以下软件：

- Docker (>= 20.10)
- Docker Compose (>= 2.0) 或 Docker CLI with Compose plugin

### 2. 克隆项目

```bash
git clone <repository-url>
cd etch
```

### 3. 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 根据需要修改配置
nano .env
```

### 4. 启动服务

```bash
# 一键启动所有服务（推荐）
make deploy

# 或者手动启动
docker compose up -d

# 查看服务状态
docker compose ps

# 查看日志
docker compose logs -f

# 验证部署
make verify
```

### 访问服务

部署成功后，可以通过以下地址访问各个服务：

- **Web管理界面**: http://localhost:10034
  - 用户名: `admin`
  - 密码: `admin123`

- **API Gateway**: http://localhost:10033
- **API健康检查**: http://localhost:10033/health

- **Bridge服务**:
  - WebSocket: ws://localhost:10031
  - UDP音频: udp://localhost:10032
  - 健康检查: http://localhost:10031/health

- **EchoKit Server**: <https://indie.echokit.dev> (外部托管服务)

- **数据库管理**: http://localhost:10037
  - 邮箱: `admin@echo-system.com`
  - 密码: `admin123`

- **Redis管理**: http://localhost:10038
  - 用户名: `admin`
  - 密码: `admin123`

- **MQTT Broker**: localhost:10039

## 详细配置

### 环境变量说明

参考 `.env.example` 文件中的配置说明：

```bash
# 数据库配置（容器内部连接）
DATABASE_URL=postgres://echo_user:echo_password@postgres:5432/echo_db

# Redis配置（容器内部连接）
REDIS_URL=redis://:redis_password@redis:6379

# JWT配置
JWT_SECRET=your-super-secret-jwt-key-change-in-production

# EchoKit Server配置（外部服务）
ECHOKIT_WEBSOCKET_URL=wss://indie.echokit.dev/ws/your-visitor-id
ECHOKIT_API_BASE_URL=https://indie.echokit.dev
```

### 端口映射

所有端口都配置为独立端口以避免冲突：

```yaml
services:
  api-gateway:
    image: echo-api-gateway:latest
    ports:
      - "10033:8080"  # 主机端口:容器端口
```

**说明：**

- 10034 → Web管理界面 (5174)
- 10033 → API Gateway (8080)
```

**当前端口映射：**

- 18084 → Web管理界面 (5174)
- 18080 → API Gateway (8080)
- 10031 → Bridge WebSocket (8082)
- 10032 → Bridge UDP (8083)
- 10035 → PostgreSQL (5432)
- 10036 → Redis (6379)
- 10037 → pgAdmin (80)
- 10038 → Redis Commander (8081)
- 10039 → MQTT Broker (1883)
- 10040 → MQTT WebSocket (9001)

### 数据持久化

数据通过 Docker volumes 持久化：

- `postgres_data`: PostgreSQL数据
- `redis_data`: Redis数据
- `pgadmin_data`: pgAdmin配置
- `mqtt_data`: MQTT数据

## 管理命令

### Makefile 便捷命令（推荐）

```bash
# 查看所有可用命令
make help

# 基础操作
make deploy          # 完整部署（构建+启动+验证）
make up              # 启动所有服务
make down            # 停止所有服务
make restart         # 重启所有服务
make health          # 检查服务健康状态

# 日志管理
make logs            # 查看所有服务日志
make logs-api        # 查看 API Gateway 日志
make logs-bridge     # 查看 Bridge 服务日志

# 数据库和缓存操作
make db-connect      # 连接数据库
make db-backup       # 备份数据库
make redis-connect   # 连接 Redis
make redis-flush     # 清空 Redis 缓存

# 服务测试
make test-api        # 测试 API Gateway 连接
make test-web        # 测试 Web 界面连接
make test-bridge     # 测试 Bridge 服务连接
make test-echokit    # 测试外部 EchoKit Server 连接

# 系统信息
make info            # 显示系统信息
make urls            # 显示所有访问地址
make ports           # 显示端口映射
```

### Docker Compose 原生命令（推荐使用 Makefile）

```bash
# 启动所有服务
docker compose up -d

# 停止所有服务
docker compose down

# 重启特定服务
docker compose restart api-gateway

# 查看服务日志
docker compose logs -f api-gateway

# 进入容器
docker compose exec api-gateway sh
```

### 数据库管理

```bash
# 连接PostgreSQL
docker compose exec postgres psql -U echo_user -d echo_db

# 备份数据库
docker compose exec postgres pg_dump -U echo_user echo_db > backup.sql

# 恢复数据库
docker compose exec -T postgres psql -U echo_user echo_db < backup.sql
```

### Redis管理

```bash
# 连接Redis
docker compose exec redis redis-cli -a redis_password

# 清空缓存
docker compose exec redis redis-cli -a redis_password FLUSHALL
```

## 开发模式

### 启动开发环境

```bash
# 开发模式启动（挂载源码卷）
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```

### 热重载配置

开发模式下，代码修改会自动重新构建：

```yaml
# docker compose.dev.yml
services:
  api-gateway:
    volumes:
      - ./api-gateway:/app
      - /app/target  # 排除构建缓存
```

## 生产部署

### 安全配置

1. **更改默认密码**：

   ```bash
   # 修改 .env 中的密码
   POSTGRES_PASSWORD=your-secure-password
   JWT_SECRET=your-secure-jwt-secret
   ```

2. **启用HTTPS**：

   ```yaml
   # 在 nginx 配置中启用 SSL
   server {
       listen 443 ssl;
       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;
   }
   ```

3. **网络隔离**：

   ```yaml
   # 使用自定义网络
   networks:
     echo-network:
       driver: bridge
       internal: true  # 内部网络
   ```

### 性能优化

1. **资源限制**：

   ```yaml
   services:
     api-gateway:
       deploy:
         resources:
           limits:
             cpus: '1.0'
             memory: 512M
   ```

2. **健康检查**：

   ```yaml
   services:
     api-gateway:
       healthcheck:
         test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
         interval: 30s
         timeout: 10s
         retries: 3
   ```

### 监控和日志

1. **日志收集**：

   ```yaml
   logging:
     driver: "json-file"
     options:
       max-size: "10m"
       max-file: "3"
   ```

2. **监控集成**：

   ```yaml
   # 可选：集成 Prometheus/Grafana
   monitoring:
     image: prom/prometheus
     ports:
       - "9090:9090"
   ```

## 故障排除

### 常见问题

1. **服务启动失败**：

   ```bash
   # 检查服务状态
   docker compose ps

   # 查看错误日志
   docker compose logs service-name
   ```

2. **数据库连接失败**：

   ```bash
   # 检查网络连接
   docker compose exec api-gateway ping postgres

   # 验证数据库配置
   docker compose exec postgres pg_isready -U echo_user
   ```

3. **端口冲突**：

   ```bash
   # 检查端口占用
   lsof -i :10033

   # 修改 docker-compose.yml 中的端口映射
   ```

### 清理和重置

```bash
# 完全清理（包括数据）
docker compose down -v

# 清理未使用的镜像和容器
docker system prune -a

# 重新初始化
docker compose up -d --force-recreate
```

## 更新和升级

### 应用更新

```bash
# 拉取最新代码
git pull

# 重新构建并部署
docker compose build --no-cache
docker compose up -d
```

### 数据库迁移

```bash
# 运行数据库迁移
docker compose exec api-gateway ./migration-scripts/run-migrations.sh
```

## 支持和帮助

如遇到问题，请：

1. 检查日志文件
2. 验证环境配置
3. 查看服务健康状态
4. 参考故障排除章节

更多信息请参考项目文档。

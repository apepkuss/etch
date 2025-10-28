.PHONY: help build up down restart logs clean health dev prod backup restore

help: ## 显示帮助信息
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# 检测可用的 Docker Compose 命令
COMPOSE_CMD := $(shell command -v docker-compose 2>/dev/null || echo "docker compose")

# Docker Compose 基础命令
build: ## 构建所有服务镜像
	@echo "构建所有服务镜像..."
	$(COMPOSE_CMD) build

up: ## 启动所有服务
	@echo "启动所有服务..."
	$(COMPOSE_CMD) up -d
	@make health

down: ## 停止所有服务
	@echo "停止所有服务..."
	$(COMPOSE_CMD) down

restart: ## 重启所有服务
	@echo "重启所有服务..."
	$(COMPOSE_CMD) restart
	@make health

logs: ## 查看服务日志
	@echo "查看服务日志..."
	$(COMPOSE_CMD) logs -f

logs-api: ## 查看 API Gateway 日志
	@echo "查看 API Gateway 日志..."
	$(COMPOSE_CMD) logs -f api-gateway

logs-bridge: ## 查看 Bridge 服务日志
	@echo "查看 Bridge 服务日志..."
	$(COMPOSE_CMD) logs -f bridge

logs-db: ## 查看数据库日志
	@echo "查看数据库日志..."
	$(COMPOSE_CMD) logs -f postgres

logs-redis: ## 查看 Redis 日志
	@echo "查看 Redis 日志..."
	$(COMPOSE_CMD) logs -f redis

logs-echokit: ## 查看 EchoKit Server 日志
	@echo "查看 EchoKit Server 日志..."
	$(COMPOSE_CMD) logs -f echokit-server

clean: ## 清理容器和镜像
	@echo "清理容器和镜像..."
	$(COMPOSE_CMD) down -v
	docker system prune -f

health: ## 检查服务健康状态
	@echo "检查服务健康状态..."
	@sleep 5
	$(COMPOSE_CMD) ps

# EchoKit Server 相关命令
download-echokit: ## 下载 EchoKit Server
	@echo "下载 EchoKit Server..."
	./scripts/download-echokit-server.sh latest

update-echokit: ## 更新 EchoKit Server 到最新版本
	@echo "更新 EchoKit Server..."
	rm -rf echokit-server-deployment
	./scripts/download-echokit-server.sh latest

# 数据库操作
db-connect: ## 连接数据库
	@echo "连接 PostgreSQL 数据库..."
	$(COMPOSE_CMD) exec postgres psql -U echo_user -d echo_db

db-backup: ## 备份数据库
	@echo "备份数据库..."
	$(COMPOSE_CMD) exec postgres pg_dump -U echo_user echo_db > backup-$$(date +%Y%m%d-%H%M%S).sql

db-restore: ## 恢复数据库
	@echo "恢复数据库..."
	@if [ -z "$(BACKUP_FILE)" ]; then \
		echo "请指定备份文件: make db-restore BACKUP_FILE=backup-xxx.sql"; \
	else \
		$(COMPOSE_CMD) exec -T postgres psql -U echo_user echo_db < $$BACKUP_FILE; \
	fi

# Redis 操作
redis-connect: ## 连接 Redis
	@echo "连接 Redis..."
	$(COMPOSE_CMD) exec redis redis-cli -a redis_password

redis-flush: ## 清空 Redis 缓存
	@echo "清空 Redis 缓存..."
	$(COMPOSE_CMD) exec redis redis-cli -a redis_password FLUSHALL

# 服务扩展命令
scale-api: ## 扩展 API Gateway (示例: make scale-api REPLICAS=3)
	@echo "扩展 API Gateway..."
	@if [ -z "$(REPLICAS)" ]; then \
		echo "请指定副本数: make scale-api REPLICAS=3"; \
	else \
		$(COMPOSE_CMD) up -d --scale api-gateway=$(REPLICAS); \
	fi

scale-bridge: ## 扩展 Bridge 服务 (示例: make scale-bridge REPLICAS=2)
	@echo "扩展 Bridge 服务..."
	@if [ -z "$(REPLICAS)" ]; then \
		echo "请指定副本数: make scale-bridge REPLICAS=2"; \
	else \
		$(COMPOSE_CMD) up -d --scale bridge=$(REPLICAS); \
	fi

# 开发环境命令
dev: ## 开发环境启动
	@echo "启动开发环境..."
	@echo "注意：开发环境需要挂载源码目录"
	@echo "请手动修改 docker-compose.yml 添加 volume 映射"

dev-api: ## 开发模式启动 API Gateway
	@echo "开发模式启动 API Gateway..."
	cd api-gateway && cargo run

dev-bridge: ## 开发模式启动 Bridge
	@echo "开发模式启动 Bridge..."
	cd bridge && cargo run

dev-web: ## 开发模式启动 Web 界面
	@echo "开发模式启动 Web 界面..."
	cd echo-web-management && npm run dev

# 生产环境命令
prod: ## 生产环境启动
	@echo "启动生产环境..."
	@if [ -f "docker-compose.prod.yml" ]; then \
		$(COMPOSE_CMD) -f docker-compose.yml -f docker-compose.prod.yml up -d; \
	else \
		echo "未找到 docker-compose.prod.yml，使用默认配置"; \
		$(COMPOSE_CMD) up -d; \
	fi

# 监控和调试命令
status: ## 显示详细服务状态
	@echo "显示详细服务状态..."
	$(COMPOSE_CMD) ps
	@echo ""
	@echo "资源使用情况:"
	docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}"

shell-api: ## 进入 API Gateway 容器
	@echo "进入 API Gateway 容器..."
	$(COMPOSE_CMD) exec api-gateway sh

shell-bridge: ## 进入 Bridge 服务容器
	@echo "进入 Bridge 服务容器..."
	$(COMPOSE_CMD) exec bridge sh

shell-db: ## 进入数据库容器
	@echo "进入数据库容器..."
	$(COMPOSE_CMD) exec postgres sh

shell-redis: ## 进入 Redis 容器
	@echo "进入 Redis 容器..."
	$(COMPOSE_CMD) exec redis sh

# 网络和连接测试
test-api: ## 测试 API Gateway 连接
	@echo "测试 API Gateway 连接..."
	curl -f http://localhost:9031/health || echo "API Gateway 连接失败"

test-web: ## 测试 Web 界面连接
	@echo "测试 Web 界面连接..."
	curl -f http://localhost:9030/health || echo "Web 界面连接失败"

test-db: ## 测试数据库连接
	@echo "测试数据库连接..."
	$(COMPOSE_CMD) exec postgres pg_isready -U echo_user -d echo_db || echo "数据库连接失败"

test-redis: ## 测试 Redis 连接
	@echo "测试 Redis 连接..."
	$(COMPOSE_CMD) exec redis redis-cli -a redis_password ping || echo "Redis 连接失败"

test-echokit: ## 测试 EchoKit Server 连接
	@echo "测试 EchoKit Server 连接..."
	curl -f http://localhost:9034/health || echo "EchoKit Server 连接失败"

# 安全和维护命令
security-check: ## 安全检查
	@echo "执行安全检查..."
	@echo "检查容器是否以 root 用户运行..."
	$(COMPOSE_CMD) exec api-gateway id || echo "API Gateway 容器无法连接"
	@echo "检查默认密码是否仍在使用..."
	grep -q "your-super-secret-jwt-key-change-in-production" .env && echo "警告: 请修改默认 JWT 密钥" || echo "JWT 密钥已修改"

logs-cleanup: ## 清理旧日志
	@echo "清理 Docker 日志..."
	docker system prune -f --filter "until=24h"

update-images: ## 更新基础镜像
	@echo "更新 Docker 基础镜像..."
	$(COMPOSE_CMD) pull
	docker image prune -f

# 系统信息命令
info: ## 显示系统信息
	@echo "=== Echo System 信息 ==="
	@echo "Docker 版本: $$(docker --version)"
	@echo "Docker Compose 版本: $$($(COMPOSE_CMD) --version)"
	@echo "系统信息: $$(uname -a)"
	@echo ""
	@echo "=== 服务列表 ==="
	$(COMPOSE_CMD) ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"

ports: ## 显示端口映射
	@echo "=== 端口映射 ==="
	@echo "Web 管理界面:  http://localhost:9030"
	@echo "API Gateway:      http://localhost:9031"
	@echo "EchoKit Server:   http://localhost:9034"
	@echo "PostgreSQL:       localhost:5432"
	@echo "Redis:           localhost:6379"
	@echo "pgAdmin:         http://localhost:9035"
	@echo "Redis Commander:  http://localhost:9036"
	@echo "MQTT:           localhost:9037"
	@echo ""

urls: ## 显示所有访问 URL
	@echo "=== 访问地址 ==="
	@echo "📱 Web管理界面:    http://localhost:9030"
	@echo "     用户名: admin, 密码: admin123"
	@echo "🔌 API Gateway:    http://localhost:9031"
	@echo "🧠 EchoKit Server: http://localhost:9034"
	@echo "🗄️  数据库管理:     http://localhost:9035"
	@echo "     邮箱: admin@echo-system.com, 密码: admin123"
	@echo "💾 Redis管理:      http://localhost:9036"
	@echo "     用户名: admin, 密码: admin123"
	@echo "📡 MQTT Broker:    localhost:9037"
	@echo ""

# 部署相关命令
verify: ## 验证完整部署
	@echo "验证完整部署..."
	./verify-deployment.sh

deploy: ## 完整部署流程
	@echo "开始完整部署流程..."
	make build
	make up
	make verify

reset: ## 完全重置系统（危险操作）
	@echo "警告：这将删除所有容器、网络和数据！"
	@echo "输入 'RESET' 来确认: "
	@read confirmation && \
	if [ "$$confirmation" = "RESET" ]; then \
		make clean; \
		rm -rf echokit-server-deployment downloads; \
		echo "系统已完全重置"; \
	else \
		echo "操作已取消"; \
	fi

# 快速操作命令
quick-restart-api: ## 快速重启 API Gateway
	@echo "快速重启 API Gateway..."
	$(COMPOSE_CMD) restart api-gateway

quick-restart-bridge: ## 快速重启 Bridge
	@echo "快速重启 Bridge..."
	$(COMPOSE_CMD) restart bridge

quick-restart-web: ## 快速重启 Web 界面
	@echo "快速重启 Web 界面..."
	$(COMPOSE_CMD) restart web-management

view-api-logs: ## 查看 API Gateway 最新日志
	@echo "查看 API Gateway 最新日志..."
	$(COMPOSE_CMD) logs --tail=100 api-gateway

view-bridge-logs: ## 查看 Bridge 最新日志
	@echo "查看 Bridge 最新日志..."
	$(COMPOSE_CMD) logs --tail=100 bridge
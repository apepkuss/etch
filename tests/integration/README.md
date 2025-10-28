# Echo System 集成测试

本目录包含 Echo System 的集成测试套件，用于验证系统各个组件之间的集成是否正常工作。

## 📋 测试套件概览

### 1. 用户界面与 API Gateway 集成测试
- **脚本**: `test_web_api_integration.sh`
- **目的**: 验证 Web 管理界面与 API Gateway 之间的通信
- **测试内容**:
  - API Gateway 健康检查
  - Web 界面健康检查
  - 设备列表 API 端点
  - CORS 头配置
  - 仪表板数据获取
  - 静态资源访问

### 2. API Gateway 与存储层集成测试
- **脚本**: `test_api_storage_integration.sh`
- **目的**: 验证 API Gateway 与 PostgreSQL、Redis 的集成
- **测试内容**:
  - PostgreSQL 数据库连接
  - Redis 缓存连接
  - 数据库表结构验证
  - 默认数据验证
  - API 数据库操作
  - Redis 缓存操作
  - 会话存储
  - Cache-Aside 模式
  - 事务回滚

### 3. 完整集成测试运行器
- **脚本**: `run_all_tests.sh`
- **目的**: 运行所有集成测试并生成报告
- **功能**:
  - 自动部署服务
  - 依次执行所有测试套件
  - 生成测试报告
  - 清理测试环境

## 🚀 快速开始

### 前置要求

- Docker 和 Docker Compose
- curl 命令
- 足够的系统资源（建议 4GB+ 内存）

### 运行所有测试

```bash
# 运行完整集成测试
./tests/integration/run_all_tests.sh

# 保留服务用于调试
./tests/integration/run_all_tests.sh --keep-services

# 跳过部署（服务已运行）
./tests/integration/run_all_tests.sh --skip-deployment

# 设置自定义超时时间
./tests/integration/run_all_tests.sh --timeout 900
```

### 运行单个测试套件

```bash
# 用户界面与 API Gateway 集成测试
./tests/integration/test_web_api_integration.sh

# API Gateway 与存储层集成测试
./tests/integration/test_api_storage_integration.sh

# 使用自定义参数
./tests/integration/test_web_api_integration.sh \
  --api-url http://localhost:9031 \
  --web-url http://localhost:9030 \
  --timeout 300
```

## 📊 测试报告

测试完成后会生成详细的测试报告，包括：

- 测试执行时间
- 测试结果统计
- 系统信息
- 失败原因（如有）

报告文件示例：`integration-test-report-20241228-143022.txt`

## 🔧 本地开发测试

### 1. 准备环境

```bash
# 确保服务已停止
docker compose down -v

# 复制环境变量文件
cp .env.example .env
```

### 2. 启动服务

```bash
# 下载 EchoKit Server
./scripts/download-echokit-server.sh latest

# 启动所有服务
make deploy

# 验证服务状态
make health
```

### 3. 运行测试

```bash
# 运行特定测试
./tests/integration/test_web_api_integration.sh

# 调试模式（保留服务）
./tests/integration/run_all_tests.sh --keep-services
```

### 4. 查看日志

```bash
# 查看所有服务日志
docker compose logs -f

# 查看特定服务日志
docker compose logs -f api-gateway
docker compose logs -f postgres
docker compose logs -f redis
```

## 🐛 故障排除

### 常见问题

1. **服务启动失败**
   ```bash
   # 检查服务状态
   docker compose ps

   # 查看错误日志
   docker compose logs api-gateway
   ```

2. **端口冲突**
   ```bash
   # 检查端口占用
   lsof -i :9031

   # 修改端口配置
   nano .env
   ```

3. **数据库连接失败**
   ```bash
   # 检查数据库连接
   docker compose exec postgres pg_isready -U echo_user -d echo_db

   # 查看数据库日志
   docker compose logs postgres
   ```

4. **测试超时**
   ```bash
   # 增加超时时间
   ./tests/integration/run_all_tests.sh --timeout 1200
   ```

### 调试技巧

1. **启用详细日志**
   ```bash
   export RUST_LOG=debug
   docker compose up -d
   ```

2. **进入容器调试**
   ```bash
   docker compose exec api-gateway sh
   docker compose exec postgres psql -U echo_user -d echo_db
   ```

3. **手动测试 API**
   ```bash
   curl http://localhost:9031/health
   curl http://localhost:9030
   ```

## 🔄 GitHub Actions 工作流

### 自动触发条件

- Push 到 `main` 或 `develop` 分支
- Pull Request 到 `main` 分支
- 每日定时运行（凌晨 2 点）
- 手动触发

### 工作流阶段

1. **代码质量检查**
   - Rust 格式检查 (`cargo fmt`)
   - Rust 代码检查 (`cargo clippy`)
   - Node.js 代码检查和构建

2. **构建测试**
   - 构建 Docker 镜像
   - 下载 EchoKit Server

3. **集成测试**
   - 用户界面与 API Gateway 集成测试
   - API Gateway 与存储层集成测试

4. **端到端测试**
   - 完整系统集成测试

5. **部署状态检查**
   - 验证所有测试通过
   - 生成部署就绪状态

### 测试结果

- 成功：绿色 ✅
- 失败：红色 ❌，包含详细日志
- 测试报告自动上传为 artifacts

## 📝 添加新测试

### 1. 创建测试脚本

```bash
# 创建新测试脚本
touch tests/integration/test_new_feature.sh
chmod +x tests/integration/test_new_feature.sh
```

### 2. 测试脚本模板

```bash
#!/bin/bash
# 新功能集成测试

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 测试函数
test_new_feature() {
    log_info "测试新功能..."

    # 实现测试逻辑
    if [condition]; then
        log_success "新功能测试通过"
        return 0
    else
        log_error "新功能测试失败"
        return 1
    fi
}

# 主函数
main() {
    log_info "开始新功能集成测试"

    if test_new_feature; then
        log_success "🎉 新功能集成测试通过！"
    else
        log_error "❌ 新功能集成测试失败"
        exit 1
    fi
}

main "$@"
```

### 3. 更新工作流

在 `.github/workflows/test.yml` 中添加新的测试套件：

```yaml
strategy:
  matrix:
    test-suite: [
      { name: "Web-API Integration", script: "test_web_api_integration.sh" },
      { name: "API-Storage Integration", script: "test_api_storage_integration.sh" },
      { name: "New Feature Integration", script: "test_new_feature.sh" }
    ]
```

## 📚 相关文档

- [DEPLOYMENT.md](../../DEPLOYMENT.md) - 部署指南
- [README-DOCKER.md](../../README-DOCKER.md) - Docker 部署说明
- [Makefile](../../Makefile) - 便捷命令

## 🤝 贡献指南

1. 为新功能添加相应的集成测试
2. 确保所有测试都能在本地和 CI 环境中运行
3. 遵循现有的测试脚本格式和命名规范
4. 添加适当的错误处理和日志记录
5. 更新相关文档

---

如有问题或建议，请提交 Issue 或 Pull Request。
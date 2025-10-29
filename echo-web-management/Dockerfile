# 多阶段构建 - 构建阶段
FROM node:18-alpine AS builder

# 设置工作目录
WORKDIR /app

# 复制 package.json 和 package-lock.json (如果存在)
COPY package*.json ./

# 安装所有依赖（包括开发依赖，用于构建）
RUN npm ci

# 复制源代码
COPY . .

# 构建应用（在CI环境中跳过TypeScript类型检查）
RUN npm run build:ci

# 生产阶段 - 使用 nginx 服务静态文件
FROM nginx:alpine AS production

# 安装必要的工具
RUN apk add --no-cache curl

# 创建非 root 用户
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001

# 复制自定义 nginx 配置
COPY nginx.conf /etc/nginx/nginx.conf

# 从构建阶段复制构建产物
COPY --from=builder /app/dist /usr/share/nginx/html

# 创建 nginx 日志目录
RUN mkdir -p /var/log/nginx && \
    chown -R nextjs:nodejs /var/log/nginx && \
    chown -R nextjs:nodejs /usr/share/nginx/html && \
    chown -R nextjs:nodejs /var/cache/nginx && \
    chown -R nextjs:nodejs /var/run/nginx.pid

# 切换到非 root 用户
USER nextjs

# 暴露端口
EXPOSE 5174

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:5174/ || exit 1

# 启动 nginx
CMD ["nginx", "-g", "daemon off;"]
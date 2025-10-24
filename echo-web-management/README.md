# Echo Web Management

基于 React 18 + TypeScript + Vite 的智能音箱Web管理界面MVP项目。

## 🚀 项目特性

- **现代化技术栈**：React 18 + TypeScript + Vite + Ant Design
- **实时通信**：WebSocket实时设备状态更新
- **响应式设计**：支持桌面端和移动端访问
- **类型安全**：完整的TypeScript类型定义
- **组件化架构**：模块化的组件设计
- **状态管理**：基于Zustand的轻量级状态管理
- **Mock数据**：完整的模拟API和数据支持

## 📋 功能模块

### 🏠 仪表板
- 设备状态概览和统计
- 实时设备在线状态监控
- 活跃会话显示
- 设备分布统计图表

### 📱 设备管理
- 设备列表展示和搜索
- 设备状态实时更新
- 设备详情查看和配置
- 设备远程控制（重启、语音测试等）
- 设备添加和删除

### 💬 会话记录
- 历史会话查询和筛选
- 会话详情查看
- 实时会话监控
- 会话统计分析

### ⚙️ 系统设置
- 系统参数配置
- 通知设置管理
- API服务配置
- 安全设置选项

## 🛠️ 技术栈

### 前端框架
- **React 18**：用户界面构建
- **TypeScript**：类型安全的JavaScript
- **Vite**：现代化构建工具
- **Ant Design**：企业级UI组件库

### 状态管理
- **Zustand**：轻量级状态管理
- **TanStack Query**：服务端状态管理

### 路由和导航
- **React Router v6**：客户端路由

### 实时通信
- **Socket.IO Client**：WebSocket客户端
- **Axios**：HTTP客户端

### 工具库
- **dayjs**：日期时间处理
- **lodash**：工具函数库

## 🚀 快速开始

### 环境要求
- Node.js >= 16.0.0
- npm >= 8.0.0

### 安装依赖
```bash
npm install
```

### 开发环境启动
```bash
npm run dev
```

### 构建生产版本
```bash
npm run build
```

### 预览生产版本
```bash
npm run preview
```

## 📁 项目结构

```
echo-web-management/
├── public/                 # 静态资源
├── src/
│   ├── components/         # 通用组件
│   │   └── Layout.tsx     # 主布局组件
│   ├── hooks/             # 自定义Hooks
│   │   └── useWebSocket.ts # WebSocket通信Hook
│   ├── pages/             # 页面组件
│   │   ├── Dashboard.tsx  # 仪表板
│   │   ├── DeviceList.tsx # 设备列表
│   │   ├── DeviceDetail.tsx # 设备详情
│   │   ├── Sessions.tsx   # 会话记录
│   │   ├── Settings.tsx   # 系统设置
│   │   └── Login.tsx      # 登录页面
│   ├── router/            # 路由配置
│   │   └── index.tsx
│   ├── stores/            # 状态管理
│   │   ├── useDeviceStore.ts # 设备状态管理
│   │   └── useSessionStore.ts # 会话状态管理
│   ├── types/             # TypeScript类型定义
│   │   └── index.ts
│   ├── App.tsx            # 根组件
│   ├── App.css            # 全局样式
│   └── main.tsx           # 应用入口
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## 🔧 开发指南

### 状态管理

项目使用 Zustand 进行状态管理，主要包含两个store：

1. **useDeviceStore**：设备相关状态
   - 设备列表数据
   - 设备状态更新
   - 设备操作（添加、删除、重启等）

2. **useSessionStore**：会话相关状态
   - 会话记录数据
   - 活跃会话管理
   - 会话统计信息

### 实时通信

使用 Socket.IO 实现WebSocket实时通信：

```typescript
import { useWebSocket } from './hooks/useWebSocket';

const { isConnected, sendMessage } = useWebSocket({
  url: 'ws://localhost:8080',
  autoConnect: true
});
```

### 类型定义

项目采用完整的TypeScript类型定义：

```typescript
interface Device {
  id: string;
  name: string;
  type: DeviceType;
  status: DeviceStatus;
  location: string;
  // ... 更多属性
}
```

## 📊 演示数据

项目包含完整的Mock数据，用于演示和测试：

- 模拟设备数据（智能音箱、显示屏、中控设备）
- 模拟会话记录
- 实时状态变化模拟
- API接口模拟

## 🎨 UI组件

使用Ant Design作为UI组件库，主要特性：

- 响应式设计，支持多端适配
- 丰富的组件库，开箱即用
- 完善的主题定制能力
- 国际化支持

## 🔐 模拟登录

项目提供模拟登录功能：

- 用户名：`admin`
- 密码：`admin123`

## 📱 响应式设计

采用响应式设计，支持：

- 桌面端（>=1200px）
- 平板端（768px-1199px）
- 移动端（<768px）

## 🚀 部署说明

### 开发环境
```bash
npm run dev
```

### 生产构建
```bash
npm run build
```

构建产物位于 `dist/` 目录，可部署到任何静态文件服务器。

### Docker部署（可选）
```dockerfile
FROM node:18-alpine as builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

## 🔄 API集成

项目预留了完整的API集成接口：

- 设备管理API
- 会话记录API
- 用户认证API
- 配置管理API

可根据实际后端API进行对接。

## 🤝 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [React](https://reactjs.org/) - 用户界面库
- [TypeScript](https://www.typescriptlang.org/) - 类型安全的JavaScript
- [Vite](https://vitejs.dev/) - 下一代前端构建工具
- [Ant Design](https://ant.design/) - 企业级UI设计语言
- [Zustand](https://github.com/pmndrs/zustand) - 简单的状态管理

## 📞 联系方式

如有问题或建议，请通过以下方式联系：

- 项目Issues：[GitHub Issues](https://github.com/your-repo/issues)
- 邮箱：your-email@example.com

---

⭐ 如果这个项目对你有帮助，请给它一个星标！

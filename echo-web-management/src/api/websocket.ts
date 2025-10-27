import { WebSocketMessage, NotificationLevel, DeviceStatus, SessionStage } from '../types';

export interface WebSocketCallbacks {
  onConnect?: () => void;
  onDisconnect?: () => void;
  onMessage?: (message: WebSocketMessage) => void;
  onError?: (error: Event) => void;
}

class WebSocketService {
  private ws: WebSocket | null = null;
  private url: string;
  private callbacks: WebSocketCallbacks = {};
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectInterval = 5000;

  constructor(url: string = 'ws://localhost:8080/ws') {
    this.url = url;
  }

  // 连接 WebSocket
  connect(callbacks: WebSocketCallbacks = {}): void {
    this.callbacks = callbacks;

    try {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.reconnectAttempts = 0;
        this.callbacks.onConnect?.();
      };

      this.ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data);
          this.handleMessage(message);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      this.ws.onclose = (event) => {
        console.log('WebSocket disconnected:', event.code, event.reason);
        this.callbacks.onDisconnect?.();
        this.handleReconnect();
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        this.callbacks.onError?.(error);
      };
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      this.callbacks.onError?.(error as Event);
    }
  }

  // 断开连接
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  // 发送消息
  send(message: any): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket is not connected');
    }
  }

  // 发送 ping 消息
  ping(): void {
    this.send({ type: 'ping', timestamp: Date.now() });
  }

  // 处理接收到的消息
  private handleMessage(message: WebSocketMessage): void {
    console.log('Received WebSocket message:', message);

    // 处理系统通知
    if (message.SystemNotification) {
      this.handleSystemNotification(message.SystemNotification);
    }

    // 处理设备状态更新
    if (message.DeviceStatusUpdate) {
      this.handleDeviceStatusUpdate(message.DeviceStatusUpdate);
    }

    // 处理会话进度更新
    if (message.SessionProgress) {
      this.handleSessionProgress(message.SessionProgress);
    }

    // 调用用户定义的回调
    this.callbacks.onMessage?.(message);
  }

  // 处理系统通知
  private handleSystemNotification(notification: {
    level: NotificationLevel;
    title: string;
    message: string;
  }): void {
    console.log(`[${notification.level}] ${notification.title}: ${notification.message}`);

    // 这里可以添加通知显示逻辑
    // 例如：使用 Ant Design 的 notification 组件
  }

  // 处理设备状态更新
  private handleDeviceStatusUpdate(update: {
    device_id: string;
    status: DeviceStatus;
    timestamp: string;
  }): void {
    console.log(`Device ${update.device_id} status changed to ${update.status}`);

    // 这里可以添加设备状态更新逻辑
    // 例如：更新设备 store 中的状态
  }

  // 处理会话进度更新
  private handleSessionProgress(progress: {
    session_id: string;
    device_id: string;
    stage: SessionStage;
    progress: number;
    message: string;
  }): void {
    console.log(`Session ${progress.session_id} progress: ${progress.progress}% - ${progress.message}`);

    // 这里可以添加会话进度更新逻辑
    // 例如：更新会话 UI 显示进度
  }

  // 处理重连逻辑
  private handleReconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      console.log(`Attempting to reconnect... (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

      setTimeout(() => {
        this.connect(this.callbacks);
      }, this.reconnectInterval);
    } else {
      console.error('Max reconnection attempts reached');
    }
  }

  // 获取连接状态
  get readyState(): number {
    return this.ws?.readyState ?? WebSocket.CLOSED;
  }

  // 检查是否已连接
  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}

// 导出单例实例
export const websocketService = new WebSocketService();

// 导出类型和服务
export { WebSocketService as default };
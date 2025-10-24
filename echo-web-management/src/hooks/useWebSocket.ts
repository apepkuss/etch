import { useEffect, useRef, useState } from 'react';
import { io, Socket } from 'socket.io-client';
import { RealtimeMessage, MessageType } from '../types';
import { useDeviceStore } from '../stores/useDeviceStore';
import { useSessionStore } from '../stores/useSessionStore';

interface UseWebSocketOptions {
  url?: string;
  autoConnect?: boolean;
  onConnect?: () => void;
  onDisconnect?: () => void;
  onError?: (error: Error) => void;
}

export const useWebSocket = (options: UseWebSocketOptions = {}) => {
  const {
    url = 'ws://localhost:8080',
    autoConnect = true,
    onConnect,
    onDisconnect,
    onError
  } = options;

  const socketRef = useRef<Socket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [lastMessage, setLastMessage] = useState<RealtimeMessage | null>(null);

  const updateDeviceStatus = useDeviceStore(state => state.updateDeviceStatus);
  const addSession = useSessionStore(state => state.addSession);
  const updateSession = useSessionStore(state => state.updateSession);

  useEffect(() => {
    if (!autoConnect) return;

    // 创建Socket连接
    const socket = io(url, {
      transports: ['websocket', 'polling'],
      timeout: 10000,
      reconnection: true,
      reconnectionAttempts: 5,
      reconnectionDelay: 1000
    });

    socketRef.current = socket;

    // 连接事件
    socket.on('connect', () => {
      console.log('WebSocket connected');
      setIsConnected(true);
      onConnect?.();
    });

    socket.on('disconnect', (reason) => {
      console.log('WebSocket disconnected:', reason);
      setIsConnected(false);
      onDisconnect?.();
    });

    socket.on('connect_error', (error) => {
      console.error('WebSocket connection error:', error);
      onError?.(error);
    });

    // 消息处理
    socket.on('device_message', (message: RealtimeMessage) => {
      console.log('Received device message:', message);
      setLastMessage(message);
      handleMessage(message);
    });

    socket.on('session_update', (message: RealtimeMessage) => {
      console.log('Received session update:', message);
      setLastMessage(message);
      handleSessionMessage(message);
    });

    // 清理函数
    return () => {
      socket.disconnect();
    };
  }, [url, autoConnect, onConnect, onDisconnect, onError]);

  // 处理设备相关消息
  const handleMessage = (message: RealtimeMessage) => {
    switch (message.type) {
      case MessageType.DEVICE_STATUS_CHANGE:
        updateDeviceStatus(message.deviceId, message.payload.status);
        break;

      case MessageType.DEVICE_BATTERY_UPDATE:
        // 更新设备电量信息
        useDeviceStore.getState().updateDeviceConfig(message.deviceId, {
          batteryLevel: message.payload.batteryLevel
        });
        break;

      case MessageType.DEVICE_ERROR:
        updateDeviceStatus(message.deviceId, 'error');
        break;

      default:
        console.log('Unhandled message type:', message.type);
    }
  };

  // 处理会话相关消息
  const handleSessionMessage = (message: RealtimeMessage) => {
    switch (message.type) {
      case MessageType.SESSION_STARTED:
        addSession(message.payload);
        break;

      case MessageType.SESSION_UPDATED:
        updateSession(message.payload.id, message.payload);
        break;

      default:
        console.log('Unhandled session message type:', message.type);
    }
  };

  // 发送消息
  const sendMessage = (type: string, payload: any, deviceId?: string) => {
    if (!socketRef.current || !isConnected) {
      console.warn('WebSocket not connected');
      return;
    }

    const message: RealtimeMessage = {
      type: type as MessageType,
      deviceId: deviceId || '',
      timestamp: new Date().toISOString(),
      payload
    };

    socketRef.current.emit('client_message', message);
  };

  // 发送设备控制命令
  const sendDeviceCommand = (deviceId: string, command: string, params?: any) => {
    sendMessage('device_command', { command, params }, deviceId);
  };

  // 发送设备配置更新
  const updateDevice = (deviceId: string, config: any) => {
    sendMessage('device_config_update', config, deviceId);
  };

  // 重连
  const reconnect = () => {
    if (socketRef.current) {
      socketRef.current.connect();
    }
  };

  // 断开连接
  const disconnect = () => {
    if (socketRef.current) {
      socketRef.current.disconnect();
    }
  };

  return {
    isConnected,
    lastMessage,
    sendMessage,
    sendDeviceCommand,
    updateDevice,
    reconnect,
    disconnect
  };
};

// 模拟实时数据生成（用于演示）
export const useMockRealtimeData = () => {
  const updateDeviceStatus = useDeviceStore(state => state.updateDeviceStatus);
  const addSession = useSessionStore(state => state.addSession);

  useEffect(() => {
    // 模拟设备状态变化
    const statusInterval = setInterval(() => {
      const devices = ['dev001', 'dev002', 'dev003'];
      const randomDevice = devices[Math.floor(Math.random() * devices.length)];
      const statuses = ['online', 'offline', 'maintenance'];
      const randomStatus = statuses[Math.floor(Math.random() * statuses.length)];

      updateDeviceStatus(randomDevice, randomStatus as any);
    }, 15000); // 每15秒更新一次设备状态

    // 模拟新会话
    const sessionInterval = setInterval(() => {
      const devices = ['dev001', 'dev002', 'dev003'];
      const randomDevice = devices[Math.floor(Math.random() * devices.length)];

      const newSession = {
        id: `sess${Date.now()}`,
        deviceId: randomDevice,
        userId: 'user001',
        startTime: new Date().toISOString(),
        transcription: '模拟语音命令',
        response: '模拟响应内容',
        status: 'active' as const
      };

      addSession(newSession);

      // 3秒后完成会话
      setTimeout(() => {
        useSessionStore.getState().updateSession(newSession.id, {
          endTime: new Date().toISOString(),
          duration: 3,
          status: 'completed' as const
        });
      }, 3000);
    }, 12000); // 每12秒创建一个新会话

    return () => {
      clearInterval(statusInterval);
      clearInterval(sessionInterval);
    };
  }, [updateDeviceStatus, addSession]);
};
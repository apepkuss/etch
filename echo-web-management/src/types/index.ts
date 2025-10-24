// 设备相关类型定义
export interface Device {
  id: string;
  name: string;
  type: DeviceType;
  status: DeviceStatus;
  location: string;
  firmwareVersion: string;
  batteryLevel?: number;
  volume: number;
  lastSeen: string;
  isOnline: boolean;
  owner: string;
}

export const DeviceType = {
  SPEAKER: 'speaker',
  DISPLAY: 'display',
  HUB: 'hub'
} as const;

export type DeviceType = typeof DeviceType[keyof typeof DeviceType];

export const DeviceStatus = {
  ONLINE: 'online',
  OFFLINE: 'offline',
  MAINTENANCE: 'maintenance',
  ERROR: 'error'
} as const;

export type DeviceStatus = typeof DeviceStatus[keyof typeof DeviceStatus];

// 用户相关类型定义
export interface User {
  id: string;
  username: string;
  email: string;
  role: UserRole;
  createdAt: string;
  lastLogin?: string;
}

export const UserRole = {
  ADMIN: 'admin',
  USER: 'user',
  VIEWER: 'viewer'
} as const;

export type UserRole = typeof UserRole[keyof typeof UserRole];

// 会话相关类型定义
export interface Session {
  id: string;
  deviceId: string;
  userId: string;
  startTime: string;
  endTime?: string;
  duration?: number;
  transcription: string;
  response: string;
  status: SessionStatus;
}

export const SessionStatus = {
  ACTIVE: 'active',
  COMPLETED: 'completed',
  INTERRUPTED: 'interrupted'
} as const;

export type SessionStatus = typeof SessionStatus[keyof typeof SessionStatus];

// API响应类型定义
export interface ApiResponse<T> {
  success: boolean;
  data: T;
  message?: string;
  code?: number;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// 设备配置类型定义
export interface DeviceConfig {
  deviceId: string;
  volume: number;
  wakeWord: string;
  voiceSettings: VoiceSettings;
  networkSettings: NetworkSettings;
}

export interface VoiceSettings {
  language: string;
  voiceGender: 'male' | 'female';
  speechRate: number;
  pitch: number;
}

export interface NetworkSettings {
  wifiEnabled: boolean;
  bluetoothEnabled: boolean;
  autoReconnect: boolean;
}

// 实时消息类型定义
export interface RealtimeMessage {
  type: MessageType;
  deviceId: string;
  timestamp: string;
  payload: any;
}

export const MessageType = {
  DEVICE_STATUS_CHANGE: 'device_status_change',
  DEVICE_BATTERY_UPDATE: 'device_battery_update',
  SESSION_STARTED: 'session_started',
  SESSION_UPDATED: 'session_updated',
  DEVICE_ERROR: 'device_error'
} as const;

export type MessageType = typeof MessageType[keyof typeof MessageType];

// 统计数据类型定义
export interface DashboardStats {
  totalDevices: number;
  onlineDevices: number;
  totalSessions: number;
  activeSessions: number;
  averageSessionDuration: number;
  deviceTypeDistribution: Record<DeviceType, number>;
  statusDistribution: Record<DeviceStatus, number>;
}

// 表单类型定义
export interface DeviceFormData {
  name: string;
  type: DeviceType;
  location: string;
  owner: string;
}

export interface UserFormData {
  username: string;
  email: string;
  password?: string;
  role: UserRole;
}
// 设备相关类型定义
export interface Device {
  id: string;
  name: string;
  device_type: DeviceType;
  status: DeviceStatus;
  location: string;
  firmware_version: string;
  battery_level: number;
  volume: number;
  last_seen: string;
  is_online: boolean;
  owner: string;
}

export const DeviceType = {
  Speaker: 'speaker'
} as const;

export type DeviceType = typeof DeviceType[keyof typeof DeviceType];

export const DeviceStatus = {
  Online: 'Online',
  Offline: 'Offline',
  Maintenance: 'Maintenance',
  Error: 'Error',
  Pending: 'Pending',
  RegistrationExpired: 'RegistrationExpired'
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
  device_id: string;
  user_id: string;
  start_time: string;
  end_time?: string;
  duration?: number;
  transcription?: string;
  response?: string;
  status: SessionStatus;
}

export const SessionStatus = {
  Active: 'Active',
  Completed: 'Completed',
  Interrupted: 'Interrupted'
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

// WebSocket 消息类型定义
export interface WebSocketMessage {
  SystemNotification?: {
    level: NotificationLevel;
    title: string;
    message: string;
  };
  DeviceStatusUpdate?: {
    device_id: string;
    status: DeviceStatus;
    timestamp: string;
  };
  SessionProgress?: {
    session_id: string;
    device_id: string;
    stage: SessionStage;
    progress: number;
    message: string;
  };
}

export const NotificationLevel = {
  Info: 'Info',
  Warning: 'Warning',
  Error: 'Error'
} as const;

export type NotificationLevel = typeof NotificationLevel[keyof typeof NotificationLevel];

export const SessionStage = {
  Recording: 'Recording',
  Processing: 'Processing',
  Responding: 'Responding',
  Completed: 'Completed'
} as const;

export type SessionStage = typeof SessionStage[keyof typeof SessionStage];

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

// 设备注册相关类型定义
export interface DeviceRegistrationRequest {
  name: string;
  device_type: DeviceType;
  device_id?: string;           // 生成的设备ID ECHO_SN_MAC
  serial_number?: string;       // 设备序列号
  mac_address?: string;         // MAC地址
}

export interface DeviceRegistrationResponse {
  device_id: string;
  pairing_code: string;
  qr_token: string;
  qr_code_data: string; // Base64 encoded QR code image
  expires_at: string;
  device_type: DeviceType;
}

export interface DeviceVerificationRequest {
  pairing_code: string;
  device_info?: DeviceInfo;
}

export interface DeviceInfo {
  mac_address?: string;
  firmware_version?: string;
  hardware_version?: string;
  serial_number?: string;
}

export interface DeviceVerificationResponse {
  device_id: string;
  success: boolean;
  message: string;
  device_config?: DeviceConfig;
}

export interface RegistrationExtensionRequest {
  device_id: string;
  extension_duration_minutes?: number; // 默认15分钟
}

export interface RegistrationExtensionResponse {
  success: boolean;
  new_expires_at: string;
  extension_duration_minutes: number;
  message: string;
}

export interface PendingRegistration {
  device_id: string;
  device_name: string;
  device_type: DeviceType;
  location: string;
  pairing_code: string;
  registration_initiated: string;
  expires_at: string;
  attempts_count: number;
  max_attempts: number;
  registration_status: 'pending' | 'expired' | 'attempts_exceeded';
}

export const RegistrationStage = {
  Created: 'Created',
  WaitingForScan: 'WaitingForScan',
  Scanning: 'Scanning',
  Verifying: 'Verifying',
  Verified: 'Verified',
  Failed: 'Failed',
  Expired: 'Expired'
} as const;

export type RegistrationStage = typeof RegistrationStage[keyof typeof RegistrationStage];
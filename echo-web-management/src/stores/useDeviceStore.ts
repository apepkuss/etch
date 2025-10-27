import { create } from 'zustand';
import { Device, DeviceStatus, DeviceConfig, ApiResponse } from '../types';
import { devicesApi } from '../api';
import { websocketService } from '../api/websocket';

interface DeviceStore {
  // 状态
  devices: Device[];
  selectedDevice: Device | null;
  loading: boolean;
  error: string | null;

  // 设备统计
  stats: {
    total: number;
    online: number;
    offline: number;
    error: number;
  };

  // 操作
  fetchDevices: () => Promise<void>;
  selectDevice: (device: Device | null) => void;
  updateDeviceConfig: (deviceId: string, config: Partial<DeviceConfig>) => Promise<void>;
  restartDevice: (deviceId: string) => Promise<void>;
  deleteDevice: (deviceId: string) => Promise<void>;
  addDevice: (device: Omit<Device, 'id'>) => Promise<void>;
  updateDeviceStatus: (deviceId: string, status: DeviceStatus) => void;
  fetchDeviceStats: () => Promise<void>;
}

// 创建设备 store
export const useDeviceStore = create<DeviceStore>((set, get) => ({
  // 初始状态
  devices: [],
  selectedDevice: null,
  loading: false,
  error: null,
  stats: {
    total: 0,
    online: 0,
    offline: 0,
    error: 0,
  },

  // 获取设备列表
  fetchDevices: async () => {
    set({ loading: true, error: null });

    try {
      const devices = await devicesApi.getDevices();
      set({ devices, loading: false });
    } catch (error) {
      console.error('Failed to fetch devices:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to fetch devices',
        loading: false
      });
    }
  },

  // 选择设备
  selectDevice: (device: Device | null) => {
    set({ selectedDevice: device });
  },

  // 更新设备配置
  updateDeviceConfig: async (deviceId: string, config: Partial<DeviceConfig>) => {
    try {
      const updatedDevice = await devicesApi.updateDevice(deviceId, {
        volume: config.volume,
        location: config.location,
      });

      set(state => ({
        devices: state.devices.map(device =>
          device.id === deviceId ? { ...device, ...updatedDevice } : device
        ),
        selectedDevice: state.selectedDevice?.id === deviceId
          ? { ...state.selectedDevice, ...updatedDevice }
          : state.selectedDevice
      }));
    } catch (error) {
      console.error('Failed to update device config:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to update device config'
      });
    }
  },

  // 重启设备
  restartDevice: async (deviceId: string) => {
    try {
      await devicesApi.restartDevice(deviceId);
      // 可以添加成功提示
    } catch (error) {
      console.error('Failed to restart device:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to restart device'
      });
    }
  },

  // 删除设备
  deleteDevice: async (deviceId: string) => {
    try {
      await devicesApi.deleteDevice(deviceId);

      set(state => ({
        devices: state.devices.filter(device => device.id !== deviceId),
        selectedDevice: state.selectedDevice?.id === deviceId ? null : state.selectedDevice
      }));
    } catch (error) {
      console.error('Failed to delete device:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to delete device'
      });
    }
  },

  // 添加设备
  addDevice: async (device: Omit<Device, 'id'>) => {
    try {
      // 注意：当前 API Gateway 没有实现添加设备的端点，这里只是示例
      const newDevice: Device = {
        ...device,
        id: `dev_${Date.now()}`, // 临时 ID
      };

      set(state => ({
        devices: [...state.devices, newDevice]
      }));
    } catch (error) {
      console.error('Failed to add device:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to add device'
      });
    }
  },

  // 更新设备状态 (通常通过 WebSocket 消息触发)
  updateDeviceStatus: (deviceId: string, status: DeviceStatus) => {
    set(state => ({
      devices: state.devices.map(device =>
        device.id === deviceId
          ? {
              ...device,
              status,
              is_online: status === 'Online',
              last_seen: new Date().toISOString()
            }
          : device
      ),
      selectedDevice: state.selectedDevice?.id === deviceId
        ? {
            ...state.selectedDevice,
            status,
            is_online: status === 'Online',
            last_seen: new Date().toISOString()
          }
        : state.selectedDevice
    }));
  },

  // 获取设备统计
  fetchDeviceStats: async () => {
    try {
      const stats = await devicesApi.getDeviceStats();
      set({ stats });
    } catch (error) {
      console.error('Failed to fetch device stats:', error);
      // 不设置错误状态，因为统计信息不是关键功能
    }
  },
}));

// WebSocket 消息处理器
websocketService.connect({
  onConnect: () => {
    console.log('Device store connected to WebSocket');
  },
  onMessage: (message) => {
    // 处理设备状态更新
    if (message.DeviceStatusUpdate) {
      const { device_id, status } = message.DeviceStatusUpdate;
      get().updateDeviceStatus(device_id, status);
    }
  },
  onDisconnect: () => {
    console.log('Device store disconnected from WebSocket');
  },
  onError: (error) => {
    console.error('Device store WebSocket error:', error);
  },
});

export default useDeviceStore;
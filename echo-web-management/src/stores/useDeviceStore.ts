import { create } from 'zustand';
import { Device, DeviceStatus, DeviceConfig, ApiResponse } from '../types';

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
}

// Mock API函数
const mockApi = {
  async getDevices(): Promise<Device[]> {
    // 模拟API延迟
    await new Promise(resolve => setTimeout(resolve, 1000));

    return [
      {
        id: 'dev001',
        name: '客厅音箱',
        type: 'speaker' as const,
        status: 'online' as const,
        location: '客厅',
        firmwareVersion: '1.2.3',
        batteryLevel: 85,
        volume: 50,
        lastSeen: '2024-10-24T16:30:00Z',
        isOnline: true,
        owner: 'user001'
      },
      {
        id: 'dev002',
        name: '卧室显示屏',
        type: 'display' as const,
        status: 'offline' as const,
        location: '主卧室',
        firmwareVersion: '1.2.2',
        batteryLevel: 60,
        volume: 30,
        lastSeen: '2024-10-24T15:45:00Z',
        isOnline: false,
        owner: 'user001'
      },
      {
        id: 'dev003',
        name: '厨房中控',
        type: 'hub' as const,
        status: 'online' as const,
        location: '厨房',
        firmwareVersion: '1.2.3',
        batteryLevel: 95,
        volume: 40,
        lastSeen: '2024-10-24T16:32:00Z',
        isOnline: true,
        owner: 'user001'
      }
    ];
  },

  async updateDeviceConfig(deviceId: string, config: Partial<DeviceConfig>): Promise<void> {
    await new Promise(resolve => setTimeout(resolve, 500));
    console.log(`Updating device ${deviceId} with config:`, config);
  },

  async restartDevice(deviceId: string): Promise<void> {
    await new Promise(resolve => setTimeout(resolve, 2000));
    console.log(`Restarting device: ${deviceId}`);
  },

  async deleteDevice(deviceId: string): Promise<void> {
    await new Promise(resolve => setTimeout(resolve, 500));
    console.log(`Deleting device: ${deviceId}`);
  },

  async addDevice(device: Omit<Device, 'id'>): Promise<Device> {
    await new Promise(resolve => setTimeout(resolve, 800));
    return {
      ...device,
      id: `dev${Date.now()}`
    };
  }
};

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
    error: 0
  },

  // 获取设备列表
  fetchDevices: async () => {
    set({ loading: true, error: null });

    try {
      const devices = await mockApi.getDevices();

      // 计算统计数据
      const stats = {
        total: devices.length,
        online: devices.filter(d => d.status === 'online').length,
        offline: devices.filter(d => d.status === 'offline').length,
        error: devices.filter(d => d.status === 'error').length
      };

      set({ devices, stats, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to fetch devices',
        loading: false
      });
    }
  },

  // 选择设备
  selectDevice: (device) => {
    set({ selectedDevice: device });
  },

  // 更新设备配置
  updateDeviceConfig: async (deviceId, config) => {
    try {
      await mockApi.updateDeviceConfig(deviceId, config);

      // 更新本地状态
      set((state) => ({
        devices: state.devices.map(device =>
          device.id === deviceId
            ? { ...device, ...config }
            : device
        )
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to update device config'
      });
    }
  },

  // 重启设备
  restartDevice: async (deviceId) => {
    try {
      await mockApi.restartDevice(deviceId);

      // 更新设备状态
      set((state) => ({
        devices: state.devices.map(device =>
          device.id === deviceId
            ? { ...device, status: 'maintenance' as const }
            : device
        )
      }));

      // 模拟重启后恢复在线状态
      setTimeout(() => {
        set((state) => ({
          devices: state.devices.map(device =>
            device.id === deviceId
              ? { ...device, status: 'online' as const, lastSeen: new Date().toISOString() }
              : device
          )
        }));
      }, 5000);
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to restart device'
      });
    }
  },

  // 删除设备
  deleteDevice: async (deviceId) => {
    try {
      await mockApi.deleteDevice(deviceId);

      set((state) => ({
        devices: state.devices.filter(device => device.id !== deviceId),
        selectedDevice: state.selectedDevice?.id === deviceId ? null : state.selectedDevice
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to delete device'
      });
    }
  },

  // 添加设备
  addDevice: async (device) => {
    try {
      const newDevice = await mockApi.addDevice(device);

      set((state) => ({
        devices: [...state.devices, newDevice]
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to add device'
      });
    }
  },

  // 更新设备状态（用于实时更新）
  updateDeviceStatus: (deviceId, status) => {
    set((state) => {
      const updatedDevices = state.devices.map(device =>
        device.id === deviceId
          ? {
              ...device,
              status,
              isOnline: status === 'online',
              lastSeen: new Date().toISOString()
            }
          : device
      );

      // 重新计算统计
      const stats = {
        total: updatedDevices.length,
        online: updatedDevices.filter(d => d.status === 'online').length,
        offline: updatedDevices.filter(d => d.status === 'offline').length,
        error: updatedDevices.filter(d => d.status === 'error').length
      };

      return { devices: updatedDevices, stats };
    });
  }
}));
import apiClient from './client';
import { ApiResponse, Device } from '../types';

export interface DeviceStats {
  total: number;
  online: number;
  offline: number;
  error: number;
}

// 设备 API 服务
export const devicesApi = {
  // 获取设备列表
  async getDevices(): Promise<Device[]> {
    try {
      const response = await apiClient.get<ApiResponse<Device[]>>('/devices');
      return response.data.data;
    } catch (error) {
      console.error('Failed to fetch devices:', error);
      throw error;
    }
  },

  // 获取设备统计
  async getDeviceStats(): Promise<DeviceStats> {
    try {
      const response = await apiClient.get<ApiResponse<DeviceStats>>('/devices/stats');
      return response.data.data;
    } catch (error) {
      console.error('Failed to fetch device stats:', error);
      throw error;
    }
  },

  // 获取单个设备详情
  async getDevice(deviceId: string): Promise<Device> {
    try {
      const response = await apiClient.get<ApiResponse<Device>>(`/devices/${deviceId}`);
      return response.data.data;
    } catch (error) {
      console.error(`Failed to fetch device ${deviceId}:`, error);
      throw error;
    }
  },

  // 更新设备配置
  async updateDevice(deviceId: string, updates: Partial<Device>): Promise<Device> {
    try {
      const response = await apiClient.put<ApiResponse<Device>>(`/devices/${deviceId}`, updates);
      return response.data.data;
    } catch (error) {
      console.error(`Failed to update device ${deviceId}:`, error);
      throw error;
    }
  },

  // 删除设备
  async deleteDevice(deviceId: string): Promise<void> {
    try {
      await apiClient.delete(`/devices/${deviceId}`);
    } catch (error) {
      console.error(`Failed to delete device ${deviceId}:`, error);
      throw error;
    }
  },

  // 重启设备
  async restartDevice(deviceId: string): Promise<void> {
    try {
      await apiClient.post(`/devices/${deviceId}/restart`);
    } catch (error) {
      console.error(`Failed to restart device ${deviceId}:`, error);
      throw error;
    }
  },

  // 更新设备状态
  async updateDeviceStatus(deviceId: string, status: string): Promise<Device> {
    try {
      const response = await apiClient.patch<ApiResponse<Device>>(`/devices/${deviceId}/status`, { status });
      return response.data.data;
    } catch (error) {
      console.error(`Failed to update device status ${deviceId}:`, error);
      throw error;
    }
  },
};
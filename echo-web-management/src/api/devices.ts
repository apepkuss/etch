import apiClient from './client';
import { ApiResponse, Device,
  DeviceRegistrationRequest, DeviceRegistrationResponse,
  DeviceVerificationRequest, DeviceVerificationResponse,
  RegistrationExtensionRequest, RegistrationExtensionResponse,
  PendingRegistration } from '../types';

export interface DeviceStats {
  total: number;
  online: number;
  offline: number;
  error: number;
  maintenance: number;
  pending: number;
  by_type: {
    speaker: number;
    display: number;
    hub: number;
  };
}

// 设备 API 服务
export const devicesApi = {
  // 获取设备列表
  async getDevices(): Promise<Device[]> {
    try {
      const response = await apiClient.get<ApiResponse<any>>('/devices');

      // API 返回分页格式: {success: true, data: {items: [...], total, page, ...}}
      // 需要提取 items 数组
      if (response.data.data && response.data.data.items) {
        // 分页格式响应
        return response.data.data.items;
      } else if (Array.isArray(response.data.data)) {
        // 直接数组格式响应（兼容性）
        return response.data.data;
      } else {
        // 未知格式，返回空数组
        console.warn('Unknown API response format:', response.data);
        return [];
      }
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

  // ================= 设备注册相关API =================

  // 注册新设备
  async registerDevice(request: DeviceRegistrationRequest): Promise<DeviceRegistrationResponse> {
    try {
      const response = await apiClient.post<ApiResponse<DeviceRegistrationResponse>>('/devices/register', request);
      return response.data.data;
    } catch (error) {
      console.error('Failed to register device:', error);
      throw error;
    }
  },

  // 验证设备注册
  async verifyDevice(request: DeviceVerificationRequest): Promise<DeviceVerificationResponse> {
    try {
      const response = await apiClient.post<ApiResponse<DeviceVerificationResponse>>('/devices/verify', request);
      return response.data.data;
    } catch (error) {
      console.error('Failed to verify device:', error);
      throw error;
    }
  },

  // 延长注册时间
  async extendRegistration(deviceId: string, request: RegistrationExtensionRequest): Promise<RegistrationExtensionResponse> {
    try {
      const response = await apiClient.post<ApiResponse<RegistrationExtensionResponse>>(`/devices/${deviceId}/extend`, request);
      return response.data.data;
    } catch (error) {
      console.error(`Failed to extend registration for device ${deviceId}:`, error);
      throw error;
    }
  },

  // 取消注册
  async cancelRegistration(deviceId: string): Promise<void> {
    try {
      await apiClient.delete(`/devices/${deviceId}/cancel`);
    } catch (error) {
      console.error(`Failed to cancel registration for device ${deviceId}:`, error);
      throw error;
    }
  },

  // 获取待注册设备列表
  async getPendingRegistrations(): Promise<PendingRegistration[]> {
    try {
      const response = await apiClient.get<ApiResponse<PendingRegistration[]>>('/devices/pending');
      return response.data.data;
    } catch (error) {
      console.error('Failed to fetch pending registrations:', error);
      throw error;
    }
  },
};
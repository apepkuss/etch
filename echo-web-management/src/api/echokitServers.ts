import apiClient from './client';
import { ApiResponse } from '../types';

export interface EchoKitServer {
  id: number;
  user_id: string;
  server_url: string;
  created_at: string;
  updated_at: string;
}

export interface AddServerRequest {
  server_url: string;
}


// EchoKit Server API 服务
export const echokitServersApi = {
  // 获取服务器列表
  async getServers(): Promise<EchoKitServer[]> {
    try {
      const response = await apiClient.get<ApiResponse<EchoKitServer[]>>('/echokit-servers');
      return response.data.data;
    } catch (error) {
      console.error('Failed to fetch EchoKit servers:', error);
      throw error;
    }
  },

  // 添加新服务器
  async addServer(request: AddServerRequest): Promise<EchoKitServer> {
    try {
      const response = await apiClient.post<ApiResponse<EchoKitServer>>('/echokit-servers', request);
      return response.data.data;
    } catch (error) {
      console.error('Failed to add EchoKit server:', error);
      throw error;
    }
  },

  // 删除服务器
  async deleteServer(serverId: number): Promise<void> {
    try {
      await apiClient.delete(`/echokit-servers/${serverId}`);
    } catch (error) {
      console.error(`Failed to delete EchoKit server ${serverId}:`, error);
      throw error;
    }
  },

};

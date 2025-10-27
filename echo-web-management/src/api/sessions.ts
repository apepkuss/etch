import apiClient from './client';
import { ApiResponse, Session } from '../types';

export interface SessionStats {
  total: number;
  active: number;
  completed: number;
  interrupted: number;
}

// 会话 API 服务
export const sessionsApi = {
  // 获取会话列表
  async getSessions(): Promise<Session[]> {
    try {
      const response = await apiClient.get<ApiResponse<Session[]>>('/sessions');
      return response.data.data;
    } catch (error) {
      console.error('Failed to fetch sessions:', error);
      throw error;
    }
  },

  // 获取会话统计
  async getSessionStats(): Promise<SessionStats> {
    try {
      const response = await apiClient.get<ApiResponse<SessionStats>>('/sessions/stats');
      return response.data.data;
    } catch (error) {
      console.error('Failed to fetch session stats:', error);
      throw error;
    }
  },

  // 获取单个会话详情
  async getSession(sessionId: string): Promise<Session> {
    try {
      const response = await apiClient.get<ApiResponse<Session>>(`/sessions/${sessionId}`);
      return response.data.data;
    } catch (error) {
      console.error(`Failed to fetch session ${sessionId}:`, error);
      throw error;
    }
  },

  // 创建新会话
  async createSession(deviceId: string, userId: string): Promise<Session> {
    try {
      const response = await apiClient.post<ApiResponse<Session>>('/sessions', {
        device_id: deviceId,
        user_id: userId,
      });
      return response.data.data;
    } catch (error) {
      console.error('Failed to create session:', error);
      throw error;
    }
  },

  // 更新会话
  async updateSession(sessionId: string, updates: Partial<Session>): Promise<Session> {
    try {
      const response = await apiClient.put<ApiResponse<Session>>(`/sessions/${sessionId}`, updates);
      return response.data.data;
    } catch (error) {
      console.error(`Failed to update session ${sessionId}:`, error);
      throw error;
    }
  },

  // 完成会话
  async completeSession(sessionId: string, transcription: string, response: string): Promise<Session> {
    try {
      const response = await apiClient.post<ApiResponse<Session>>(`/sessions/${sessionId}/complete`, {
        transcription,
        response,
      });
      return response.data.data;
    } catch (error) {
      console.error(`Failed to complete session ${sessionId}:`, error);
      throw error;
    }
  },

  // 中断会话
  async interruptSession(sessionId: string): Promise<Session> {
    try {
      const response = await apiClient.post<ApiResponse<Session>>(`/sessions/${sessionId}/interrupt`);
      return response.data.data;
    } catch (error) {
      console.error(`Failed to interrupt session ${sessionId}:`, error);
      throw error;
    }
  },

  // 删除会话
  async deleteSession(sessionId: string): Promise<void> {
    try {
      await apiClient.delete(`/sessions/${sessionId}`);
    } catch (error) {
      console.error(`Failed to delete session ${sessionId}:`, error);
      throw error;
    }
  },
};
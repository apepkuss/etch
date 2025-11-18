import apiClient from './client';
import { ApiResponse, Session } from '../types';

export interface SessionStats {
  total: number;
  active: number;
  completed: number;
  interrupted: number;
  failed?: number;
  timeout?: number;
  today_sessions?: number;
  average_duration_seconds?: number;
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
      const response = await apiClient.get<ApiResponse<any>>('/sessions/stats');
      const apiData = response.data.data;

      // 转换API数据格式为前端期望的格式
      return {
        total: apiData.total || 0,
        active: apiData.active || 0,
        completed: apiData.completed || 0,
        interrupted: (apiData.failed || 0) + (apiData.timeout || 0),
        failed: apiData.failed,
        timeout: apiData.timeout,
        today_sessions: apiData.today_sessions,
        average_duration_seconds: apiData.average_duration_seconds,
      };
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
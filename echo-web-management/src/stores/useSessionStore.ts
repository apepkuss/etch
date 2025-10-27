import { create } from 'zustand';
import { Session, SessionStatus } from '../types';
import { sessionsApi } from '../api';
import { websocketService } from '../api/websocket';

interface SessionStore {
  // 状态
  sessions: Session[];
  activeSessions: Session[];
  loading: boolean;
  error: string | null;

  // 统计
  stats: {
    total: number;
    active: number;
    completed: number;
    interrupted: number;
  };

  // 操作
  fetchSessions: () => Promise<void>;
  fetchSessionStats: () => Promise<void>;
  createSession: (deviceId: string, userId: string) => Promise<Session | null>;
  updateSession: (sessionId: string, updates: Partial<Session>) => void;
  completeSession: (sessionId: string, transcription: string, response: string) => Promise<void>;
  interruptSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
}

// 创建会话 store
export const useSessionStore = create<SessionStore>((set, get) => ({
  // 初始状态
  sessions: [],
  activeSessions: [],
  loading: false,
  error: null,
  stats: {
    total: 0,
    active: 0,
    completed: 0,
    interrupted: 0,
  },

  // 获取会话列表
  fetchSessions: async () => {
    set({ loading: true, error: null });

    try {
      const sessions = await sessionsApi.getSessions();
      const activeSessions = sessions.filter(session => session.status === 'Active');

      set({
        sessions,
        activeSessions,
        loading: false
      });
    } catch (error) {
      console.error('Failed to fetch sessions:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to fetch sessions',
        loading: false
      });
    }
  },

  // 获取会话统计
  fetchSessionStats: async () => {
    try {
      const stats = await sessionsApi.getSessionStats();
      set({ stats });
    } catch (error) {
      console.error('Failed to fetch session stats:', error);
      // 不设置错误状态，因为统计信息不是关键功能
    }
  },

  // 创建新会话
  createSession: async (deviceId: string, userId: string) => {
    try {
      const newSession = await sessionsApi.createSession(deviceId, userId);

      set(state => ({
        sessions: [...state.sessions, newSession],
        activeSessions: newSession.status === 'Active'
          ? [...state.activeSessions, newSession]
          : state.activeSessions
      }));

      return newSession;
    } catch (error) {
      console.error('Failed to create session:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to create session'
      });
      return null;
    }
  },

  // 更新会话
  updateSession: (sessionId: string, updates: Partial<Session>) => {
    set(state => ({
      sessions: state.sessions.map(session =>
        session.id === sessionId ? { ...session, ...updates } : session
      ),
      activeSessions: state.activeSessions.map(session =>
        session.id === sessionId ? { ...session, ...updates } : session
      )
    }));
  },

  // 完成会话
  completeSession: async (sessionId: string, transcription: string, response: string) => {
    try {
      const updatedSession = await sessionsApi.completeSession(sessionId, transcription, response);

      set(state => ({
        sessions: state.sessions.map(session =>
          session.id === sessionId ? updatedSession : session
        ),
        activeSessions: state.activeSessions.filter(session => session.id !== sessionId)
      }));
    } catch (error) {
      console.error('Failed to complete session:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to complete session'
      });
    }
  },

  // 中断会话
  interruptSession: async (sessionId: string) => {
    try {
      const updatedSession = await sessionsApi.interruptSession(sessionId);

      set(state => ({
        sessions: state.sessions.map(session =>
          session.id === sessionId ? updatedSession : session
        ),
        activeSessions: state.activeSessions.filter(session => session.id !== sessionId)
      }));
    } catch (error) {
      console.error('Failed to interrupt session:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to interrupt session'
      });
    }
  },

  // 删除会话
  deleteSession: async (sessionId: string) => {
    try {
      await sessionsApi.deleteSession(sessionId);

      set(state => ({
        sessions: state.sessions.filter(session => session.id !== sessionId),
        activeSessions: state.activeSessions.filter(session => session.id !== sessionId)
      }));
    } catch (error) {
      console.error('Failed to delete session:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to delete session'
      });
    }
  },
}));

// WebSocket 消息处理器
websocketService.connect({
  onConnect: () => {
    console.log('Session store connected to WebSocket');
  },
  onMessage: (message) => {
    // 处理会话进度更新
    if (message.SessionProgress) {
      const { session_id, stage, progress, message: progressMessage } = message.SessionProgress;

      // 更新会话进度信息
      get().updateSession(session_id, {
        // 这里可以添加进度相关的字段
        // 例如：progress: progress, currentStage: stage, progressMessage
      });

      console.log(`Session ${session_id} progress: ${progress}% - ${progressMessage}`);
    }
  },
  onDisconnect: () => {
    console.log('Session store disconnected from WebSocket');
  },
  onError: (error) => {
    console.error('Session store WebSocket error:', error);
  },
});

export default useSessionStore;
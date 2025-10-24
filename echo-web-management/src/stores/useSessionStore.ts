import { create } from 'zustand';
import { Session, SessionStatus } from '../types';

interface SessionStore {
  // 状态
  sessions: Session[];
  activeSessions: Session[];
  loading: boolean;
  error: string | null;

  // 统计
  stats: {
    totalToday: number;
    activeNow: number;
    averageDuration: number;
  };

  // 操作
  fetchSessions: (deviceId?: string) => Promise<void>;
  getActiveSessions: () => void;
  addSession: (session: Session) => void;
  updateSession: (sessionId: string, updates: Partial<Session>) => void;
}

// Mock API函数
const mockSessionApi = {
  async getSessions(deviceId?: string): Promise<Session[]> {
    await new Promise(resolve => setTimeout(resolve, 800));

    const mockSessions: Session[] = [
      {
        id: 'sess001',
        deviceId: 'dev001',
        userId: 'user001',
        startTime: '2024-10-24T16:25:00Z',
        endTime: '2024-10-24T16:28:00Z',
        duration: 180,
        transcription: '今天天气怎么样',
        response: '今天天气晴朗，温度25度，适合外出活动',
        status: 'completed'
      },
      {
        id: 'sess002',
        deviceId: 'dev001',
        userId: 'user001',
        startTime: '2024-10-24T16:30:00Z',
        transcription: '播放音乐',
        response: '正在为您播放音乐',
        status: 'active'
      },
      {
        id: 'sess003',
        deviceId: 'dev003',
        userId: 'user001',
        startTime: '2024-10-24T16:15:00Z',
        endTime: '2024-10-24T16:17:00Z',
        duration: 120,
        transcription: '打开客厅灯',
        response: '已为您打开客厅的灯',
        status: 'completed'
      }
    ];

    return deviceId
      ? mockSessions.filter(session => session.deviceId === deviceId)
      : mockSessions;
  }
};

export const useSessionStore = create<SessionStore>((set, get) => ({
  // 初始状态
  sessions: [],
  activeSessions: [],
  loading: false,
  error: null,
  stats: {
    totalToday: 0,
    activeNow: 0,
    averageDuration: 0
  },

  // 获取会话列表
  fetchSessions: async (deviceId) => {
    set({ loading: true, error: null });

    try {
      const sessions = await mockSessionApi.getSessions(deviceId);

      // 计算统计数据
      const today = new Date().toDateString();
      const todaySessions = sessions.filter(
        session => new Date(session.startTime).toDateString() === today
      );

      const activeSessions = sessions.filter(
        session => session.status === 'active'
      );

      const completedSessions = sessions.filter(
        session => session.status === 'completed' && session.duration
      );

      const averageDuration = completedSessions.length > 0
        ? completedSessions.reduce((sum, session) => sum + (session.duration || 0), 0) / completedSessions.length
        : 0;

      const stats = {
        totalToday: todaySessions.length,
        activeNow: activeSessions.length,
        averageDuration: Math.round(averageDuration)
      };

      set({
        sessions,
        activeSessions,
        stats,
        loading: false
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to fetch sessions',
        loading: false
      });
    }
  },

  // 获取活跃会话
  getActiveSessions: () => {
    const { sessions } = get();
    const activeSessions = sessions.filter(session => session.status === 'active');
    set({ activeSessions });
  },

  // 添加新会话
  addSession: (session) => {
    set((state) => ({
      sessions: [session, ...state.sessions]
    }));

    // 如果是活跃会话，更新活跃会话列表
    if (session.status === 'active') {
      set((state) => ({
        activeSessions: [session, ...state.activeSessions],
        stats: {
          ...state.stats,
          activeNow: state.stats.activeNow + 1
        }
      }));
    }
  },

  // 更新会话
  updateSession: (sessionId, updates) => {
    set((state) => {
      const updatedSessions = state.sessions.map(session =>
        session.id === sessionId
          ? { ...session, ...updates }
          : session
      );

      // 重新计算活跃会话和统计
      const activeSessions = updatedSessions.filter(
        session => session.status === 'active'
      );

      const today = new Date().toDateString();
      const todaySessions = updatedSessions.filter(
        session => new Date(session.startTime).toDateString() === today
      );

      return {
        sessions: updatedSessions,
        activeSessions,
        stats: {
          ...state.stats,
          activeNow: activeSessions.length,
          totalToday: todaySessions.length
        }
      };
    });
  }
}));
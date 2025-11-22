import { create } from 'zustand';
import { echokitServersApi, EchoKitServer, AddServerRequest } from '../api/echokitServers';

interface EchoKitServerStore {
  // 状态
  servers: EchoKitServer[];
  loading: boolean;
  error: string | null;

  // 操作
  fetchServers: () => Promise<void>;
  addServer: (request: AddServerRequest) => Promise<void>;
  deleteServer: (serverId: number) => Promise<void>;
}

// 创建 EchoKit Server store
export const useEchoKitServerStore = create<EchoKitServerStore>((set) => ({
  // 初始状态
  servers: [],
  loading: false,
  error: null,

  // 获取服务器列表
  fetchServers: async () => {
    set({ loading: true, error: null });

    try {
      const servers = await echokitServersApi.getServers();
      set({ servers, loading: false });
    } catch (error) {
      console.error('Failed to fetch EchoKit servers:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to fetch servers',
        loading: false
      });
    }
  },

  // 添加服务器
  addServer: async (request: AddServerRequest) => {
    set({ loading: true, error: null });

    try {
      const newServer = await echokitServersApi.addServer(request);
      set(state => ({
        servers: [...state.servers, newServer],
        loading: false
      }));
    } catch (error) {
      console.error('Failed to add EchoKit server:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to add server',
        loading: false
      });
      throw error;
    }
  },

  // 删除服务器
  deleteServer: async (serverId: number) => {
    set({ loading: true, error: null });

    try {
      await echokitServersApi.deleteServer(serverId);
      set(state => ({
        servers: state.servers.filter(server => server.id !== serverId),
        loading: false
      }));
    } catch (error) {
      console.error('Failed to delete EchoKit server:', error);
      set({
        error: error instanceof Error ? error.message : 'Failed to delete server',
        loading: false
      });
      throw error;
    }
  },
}));

export default useEchoKitServerStore;

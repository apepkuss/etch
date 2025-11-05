use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

/// 流控配置
#[derive(Debug, Clone)]
pub struct FlowControlConfig {
    /// 每秒最大帧数
    pub max_frames_per_second: u32,
    /// 缓冲区大小（字节）
    pub buffer_size_bytes: usize,
    /// 窗口大小（帧数）
    pub window_size_frames: u32,
    /// 启用动态调整
    pub enable_dynamic_adjustment: bool,
}

impl Default for FlowControlConfig {
    fn default() -> Self {
        Self {
            max_frames_per_second: 50, // 20ms per frame
            buffer_size_bytes: 1024 * 1024, // 1MB
            window_size_frames: 100,
            enable_dynamic_adjustment: true,
        }
    }
}

/// 会话流控状态
#[derive(Debug, Clone)]
struct SessionFlowState {
    /// 当前窗口帧数
    current_window_frames: u32,
    /// 缓冲区已用大小
    buffer_used_bytes: usize,
    /// 最后重置时间
    last_reset: chrono::DateTime<chrono::Utc>,
    /// 是否阻塞
    is_blocked: bool,
}

impl Default for SessionFlowState {
    fn default() -> Self {
        Self {
            current_window_frames: 0,
            buffer_used_bytes: 0,
            last_reset: chrono::Utc::now(),
            is_blocked: false,
        }
    }
}

/// 流控管理器
pub struct FlowController {
    config: FlowControlConfig,
    states: Arc<RwLock<HashMap<String, SessionFlowState>>>,
}

impl FlowController {
    pub fn new(config: FlowControlConfig) -> Self {
        Self {
            config,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动流控监控
    pub async fn start(self: Arc<Self>) {
        info!("Starting flow controller");

        let mut interval = interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            if let Err(e) = self.reset_windows().await {
                warn!("Failed to reset flow control windows: {}", e);
            }
        }
    }

    /// 检查是否允许发送
    pub async fn can_send(
        &self,
        session_id: &str,
        frame_size_bytes: usize,
    ) -> bool {
        let mut states = self.states.write().await;
        let state = states.entry(session_id.to_string()).or_default();

        // 检查是否已阻塞
        if state.is_blocked {
            debug!("Session {} is blocked", session_id);
            return false;
        }

        // 检查帧数限制
        if state.current_window_frames >= self.config.window_size_frames {
            warn!("Session {} exceeds frame window", session_id);
            state.is_blocked = true;
            return false;
        }

        // 检查缓冲区限制
        if state.buffer_used_bytes + frame_size_bytes > self.config.buffer_size_bytes {
            warn!("Session {} exceeds buffer size", session_id);
            state.is_blocked = true;
            return false;
        }

        true
    }

    /// 记录发送
    pub async fn record_send(
        &self,
        session_id: &str,
        frame_size_bytes: usize,
    ) -> anyhow::Result<()> {
        let mut states = self.states.write().await;
        let state = states.entry(session_id.to_string()).or_default();

        state.current_window_frames += 1;
        state.buffer_used_bytes += frame_size_bytes;

        debug!(
            "Session {} sent frame: frames={}/{}, buffer={}/{}",
            session_id,
            state.current_window_frames,
            self.config.window_size_frames,
            state.buffer_used_bytes,
            self.config.buffer_size_bytes
        );

        Ok(())
    }

    /// 记录接收确认（减少缓冲区占用）
    pub async fn record_ack(
        &self,
        session_id: &str,
        frame_size_bytes: usize,
    ) -> anyhow::Result<()> {
        let mut states = self.states.write().await;

        if let Some(state) = states.get_mut(session_id) {
            state.buffer_used_bytes = state.buffer_used_bytes.saturating_sub(frame_size_bytes);

            // 如果缓冲区降到安全水位，解除阻塞
            if state.is_blocked && state.buffer_used_bytes < self.config.buffer_size_bytes / 2 {
                info!("Session {} unblocked", session_id);
                state.is_blocked = false;
            }

            debug!(
                "Session {} ack frame: buffer={}/{}",
                session_id, state.buffer_used_bytes, self.config.buffer_size_bytes
            );
        }

        Ok(())
    }

    /// 重置窗口
    async fn reset_windows(&self) -> anyhow::Result<()> {
        let now = chrono::Utc::now();
        let mut states = self.states.write().await;

        for (session_id, state) in states.iter_mut() {
            let elapsed = now.signed_duration_since(state.last_reset);

            if elapsed.num_seconds() >= 1 {
                debug!("Resetting window for session {}", session_id);
                state.current_window_frames = 0;
                state.last_reset = now;

                // 如果缓冲区空闲且被阻塞，解除阻塞
                if state.is_blocked && state.buffer_used_bytes == 0 {
                    info!("Session {} unblocked after window reset", session_id);
                    state.is_blocked = false;
                }
            }
        }

        Ok(())
    }

    /// 移除会话流控状态
    pub async fn remove_session(&self, session_id: &str) {
        let mut states = self.states.write().await;
        states.remove(session_id);
        debug!("Removed flow control state for session {}", session_id);
    }

    /// 获取会话流控统计
    pub async fn get_stats(&self, session_id: &str) -> Option<FlowControlStats> {
        let states = self.states.read().await;
        states.get(session_id).map(|state| FlowControlStats {
            session_id: session_id.to_string(),
            current_window_frames: state.current_window_frames,
            max_window_frames: self.config.window_size_frames,
            buffer_used_bytes: state.buffer_used_bytes,
            buffer_total_bytes: self.config.buffer_size_bytes,
            is_blocked: state.is_blocked,
        })
    }

    /// 获取所有会话统计
    pub async fn get_all_stats(&self) -> Vec<FlowControlStats> {
        let states = self.states.read().await;
        states
            .iter()
            .map(|(session_id, state)| FlowControlStats {
                session_id: session_id.clone(),
                current_window_frames: state.current_window_frames,
                max_window_frames: self.config.window_size_frames,
                buffer_used_bytes: state.buffer_used_bytes,
                buffer_total_bytes: self.config.buffer_size_bytes,
                is_blocked: state.is_blocked,
            })
            .collect()
    }
}

/// 流控统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowControlStats {
    pub session_id: String,
    pub current_window_frames: u32,
    pub max_window_frames: u32,
    pub buffer_used_bytes: usize,
    pub buffer_total_bytes: usize,
    pub is_blocked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_flow_control_config() {
        let config = FlowControlConfig::default();
        assert_eq!(config.max_frames_per_second, 50);
        assert_eq!(config.window_size_frames, 100);
    }

    #[tokio::test]
    async fn test_can_send_initial() {
        let controller = FlowController::new(FlowControlConfig::default());
        assert!(controller.can_send("session1", 1024).await);
    }

    #[tokio::test]
    async fn test_record_send() {
        let controller = FlowController::new(FlowControlConfig::default());

        assert!(controller.can_send("session1", 1024).await);
        controller.record_send("session1", 1024).await.unwrap();

        let stats = controller.get_stats("session1").await.unwrap();
        assert_eq!(stats.current_window_frames, 1);
        assert_eq!(stats.buffer_used_bytes, 1024);
    }

    #[tokio::test]
    async fn test_record_ack() {
        let controller = FlowController::new(FlowControlConfig::default());

        controller.record_send("session1", 2048).await.unwrap();
        controller.record_ack("session1", 1024).await.unwrap();

        let stats = controller.get_stats("session1").await.unwrap();
        assert_eq!(stats.buffer_used_bytes, 1024);
    }
}

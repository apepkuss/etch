use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Timeout,
}

/// 会话信息
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub device_id: String,
    pub echokit_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: SessionStatus,
    pub audio_frames_sent: u64,
    pub audio_frames_received: u64,
    /// 标记本轮对话是否已发送 StartChat 命令
    /// 每轮对话（从第一个音频包到Submit）需要发送一次 StartChat
    #[serde(skip)]
    pub start_chat_sent_for_current_round: bool,
    /// 🔧 方案B：存储多轮对话的转录文本（在会话结束时一次性写入数据库）
    /// 每轮对话的 ASR 文本会追加到这个 Vec 中
    #[serde(skip)]
    pub conversation_transcripts: Vec<String>,
    /// 🔧 存储多轮对话的 AI 回复文本（在会话结束时一次性写入数据库）
    /// 每轮对话的 AI 回复文本会追加到这个 Vec 中
    #[serde(skip)]
    pub conversation_responses: Vec<String>,
    /// 🔧 临时缓存：当前轮次的多条 AI 回复文本（用于合并）
    /// 在收到 EndResponse 时，合并为一条并添加到 conversation_responses
    #[serde(skip)]
    pub current_round_responses: Vec<String>,
}

/// 会话管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建会话
    pub async fn create_session(
        &self,
        session_id: String,
        device_id: String,
    ) -> anyhow::Result<()> {
        let session_info = SessionInfo {
            session_id: session_id.clone(),
            device_id: device_id.clone(),
            echokit_session_id: None,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            status: SessionStatus::Active,
            audio_frames_sent: 0,
            audio_frames_received: 0,
            start_chat_sent_for_current_round: false, // 初始化为false
            conversation_transcripts: Vec::new(), // 🔧 初始化为空数组
            conversation_responses: Vec::new(), // 🔧 初始化为空数组
            current_round_responses: Vec::new(), // 🔧 初始化当前轮次回复缓存为空
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_info);

        info!("Session {} created for device {}", session_id, device_id);
        Ok(())
    }

    /// 更新会话活动时间
    pub async fn update_activity(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Utc::now();
        }
        Ok(())
    }

    /// 增加发送帧计数
    pub async fn increment_sent_frames(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.audio_frames_sent += 1;
            session.last_activity = Utc::now();
        }
    }

    /// 增加接收帧计数
    pub async fn increment_received_frames(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.audio_frames_received += 1;
            session.last_activity = Utc::now();
        }
    }

    /// 结束会话
    pub async fn end_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Completed;
            info!("Session {} ended (sent: {}, received: {})",
                  session_id, session.audio_frames_sent, session.audio_frames_received);
        }
        Ok(())
    }

    /// 标记会话失败
    pub async fn mark_failed(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Failed;
        }
        Ok(())
    }

    /// 获取会话信息
    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// 获取设备的所有活跃会话
    pub async fn get_device_sessions(&self, device_id: &str) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.device_id == device_id && s.status == SessionStatus::Active)
            .cloned()
            .collect()
    }

    /// 获取设备的所有会话ID
    pub async fn get_sessions_by_device(&self, device_id: &str) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.device_id == device_id)
            .map(|s| s.session_id.clone())
            .collect()
    }

    /// 标记会话为超时
    pub async fn mark_timeout(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Timeout;
            info!("Session {} marked as timeout", session_id);
        }
        Ok(())
    }

    /// 清理超时会话
    pub async fn cleanup_timeout_sessions(&self, timeout_seconds: i64) -> usize {
        let now = Utc::now();
        let mut sessions = self.sessions.write().await;

        let mut timeout_sessions = Vec::new();
        for (session_id, session) in sessions.iter_mut() {
            if session.status == SessionStatus::Active {
                let duration = now.signed_duration_since(session.last_activity);
                if duration.num_seconds() > timeout_seconds {
                    session.status = SessionStatus::Timeout;
                    timeout_sessions.push(session_id.clone());
                }
            }
        }

        let count = timeout_sessions.len();
        if count > 0 {
            info!("Cleaned up {} timeout sessions", count);
        }

        count
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;

        let mut stats = SessionStats {
            total: sessions.len(),
            active: 0,
            completed: 0,
            failed: 0,
            timeout: 0,
        };

        for session in sessions.values() {
            match session.status {
                SessionStatus::Active => stats.active += 1,
                SessionStatus::Completed => stats.completed += 1,
                SessionStatus::Failed => stats.failed += 1,
                SessionStatus::Timeout => stats.timeout += 1,
            }
        }

        stats
    }

    /// 检查当前轮次是否需要发送 StartChat
    /// 返回 true 表示需要发送
    pub async fn needs_start_chat_for_round(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            !session.start_chat_sent_for_current_round
        } else {
            // 会话不存在，不需要发送
            false
        }
    }

    /// 标记当前轮次已发送 StartChat
    pub async fn mark_start_chat_sent(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.start_chat_sent_for_current_round = true;
            debug!("Marked StartChat as sent for session {}", session_id);
        }
    }

    /// 重置 StartChat 标记（在 Submit 后调用，准备下一轮对话）
    pub async fn reset_start_chat_flag(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.start_chat_sent_for_current_round = false;
            debug!("Reset StartChat flag for session {} (ready for next round)", session_id);
        }
    }

    /// 🔧 方案B：添加 ASR 转录文本到会话（在内存中累积）
    /// 每次收到 ASR 结果时调用，将文本追加到 conversation_transcripts 数组
    /// 包含去重逻辑：如果与上一轮内容相同，则跳过
    pub async fn append_transcript(&self, session_id: &str, transcript: String) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            // 去重：检查是否与上一轮重复
            let trimmed_transcript = transcript.trim();
            if let Some(last) = session.conversation_transcripts.last() {
                if last.trim() == trimmed_transcript {
                    warn!("⚠️ Duplicate transcript detected for session {}, skipping: {}",
                          session_id, trimmed_transcript);
                    return;
                }
            }

            session.conversation_transcripts.push(transcript.clone());
            session.last_activity = Utc::now();
            info!("📝 Appended transcript to session {} (total: {} turns)",
                  session_id, session.conversation_transcripts.len());
            debug!("Transcript content: {}", transcript);
        } else {
            warn!("⚠️ Attempted to append transcript to non-existent session: {}", session_id);
        }
    }

    /// 🔧 方案B：获取会话的所有转录文本（用于持久化到数据库）
    /// 返回用换行符连接的完整对话文本
    pub async fn get_full_transcript(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|session| {
            if session.conversation_transcripts.is_empty() {
                return None;
            }
            Some(session.conversation_transcripts.join("\n"))
        }).flatten()
    }

    /// 🔧 添加 AI 回复文本到会话（在内存中累积）
    /// 每次收到 StartAudio 事件时调用，将 AI 回复文本追加到当前轮次的临时缓存
    pub async fn append_response(&self, session_id: &str, response: String) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            // 添加到当前轮次的临时缓存，而不是直接添加到 conversation_responses
            session.current_round_responses.push(response.clone());
            session.last_activity = Utc::now();
            info!("🤖 Appended AI response fragment to session {} (current round: {} fragments)",
                  session_id, session.current_round_responses.len());
            debug!("Response fragment content: {}", response);
        } else {
            warn!("⚠️ Attempted to append response to non-existent session: {}", session_id);
        }
    }

    /// 🔧 获取会话的所有 AI 回复文本（用于持久化到数据库）
    /// 返回用换行符连接的完整 AI 回复文本
    pub async fn get_full_response(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|session| {
            if session.conversation_responses.is_empty() {
                return None;
            }
            Some(session.conversation_responses.join("\n"))
        }).flatten()
    }

    /// 🔧 完成当前轮次的 AI 回复（在收到 EndResponse 时调用）
    /// 将当前轮次临时缓存的多条 AI 回复合并为一条，添加到 conversation_responses
    pub async fn finalize_current_round_response(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if !session.current_round_responses.is_empty() {
                // 合并当前轮次的所有回复文本
                let merged_response = session.current_round_responses.join("");

                info!("✅ Finalizing current round response for session {} ({} fragments → 1 merged response)",
                      session_id, session.current_round_responses.len());
                debug!("Merged response content: {}", merged_response);

                // 添加到 conversation_responses
                session.conversation_responses.push(merged_response);

                // 清空当前轮次的临时缓存，准备下一轮
                session.current_round_responses.clear();

                session.last_activity = Utc::now();

                info!("📝 Session {} now has {} complete conversation rounds",
                      session_id, session.conversation_responses.len());
            } else {
                debug!("No response fragments to finalize for session {}", session_id);
            }
        } else {
            warn!("⚠️ Attempted to finalize response for non-existent session: {}", session_id);
        }
    }
}

/// 会话统计
#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub timeout: usize,
}


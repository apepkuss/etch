use anyhow::{Context, Result};
use echo_shared::{AudioFormat, AudioChunk};
use echo_shared::utils::now_utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, error, debug};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read, Write};

// 音频处理器
pub struct AudioProcessor {
    device_sessions: Arc<RwLock<HashMap<String, DeviceAudioSession>>>,
    echokit_client: Arc<crate::echokit_client::EchoKitClient>,
    output_sender: mpsc::UnboundedSender<(String, Vec<u8>)>, // (device_id, audio_data)
}

// 设备音频会话
#[derive(Debug, Clone)]
struct DeviceAudioSession {
    device_id: String,
    session_id: String,
    input_format: AudioFormat,
    output_format: AudioFormat,
    sample_rate: u32,
    channels: u8,
    buffer: Vec<u8>,
    is_recording: bool,
    last_audio_time: chrono::DateTime<chrono::Utc>,
}

impl AudioProcessor {
    pub fn new(
        echokit_client: Arc<crate::echokit_client::EchoKitClient>,
        output_sender: mpsc::UnboundedSender<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            device_sessions: Arc::new(RwLock::new(HashMap::new())),
            echokit_client,
            output_sender,
        }
    }

    // 开始设备的音频会话
    pub async fn start_session(
        &self,
        device_id: String,
        session_id: String,
        input_format: AudioFormat,
        output_format: AudioFormat,
        sample_rate: u32,
        channels: u8,
    ) -> Result<()> {
        let audio_session = DeviceAudioSession {
            device_id: device_id.clone(),
            session_id: session_id.clone(),
            input_format,
            output_format,
            sample_rate,
            channels,
            buffer: Vec::new(),
            is_recording: true,
            last_audio_time: now_utc(),
        };

        self.device_sessions.write().await.insert(device_id.clone(), audio_session);
        info!("Started audio session for device: {}", device_id);

        Ok(())
    }

    // 结束设备的音频会话
    pub async fn end_session(&self, device_id: &str, reason: &str) -> Result<()> {
        let mut sessions = self.device_sessions.write().await;
        if let Some(session) = sessions.remove(device_id) {
            // 通知 EchoKit 结束会话
            if let Err(e) = self.echokit_client.end_session(
                session.session_id.clone(),
                device_id.to_string(),
                reason.to_string(),
            ).await {
                error!("Failed to end EchoKit session: {}", e);
            }

            info!("Ended audio session for device: {}", device_id);
            Ok(())
        } else {
            warn!("No active session found for device: {}", device_id);
            Err(anyhow::anyhow!("No active session found"))
        }
    }

    // 处理来自设备的音频数据
    pub async fn process_device_audio(
        &self,
        device_id: &str,
        audio_data: Vec<u8>,
        format: AudioFormat,
    ) -> Result<()> {
        let sessions = self.device_sessions.read().await;

        if let Some(session) = sessions.get(device_id) {
            if !session.is_recording {
                debug!("Device {} is not recording, ignoring audio data", device_id);
                return Ok(());
            }

            // 转换音频格式并处理
            let processed_audio = self.convert_audio_format(
                audio_data,
                format,
                session.input_format,
                session.sample_rate,
                session.channels,
            ).await?;

            // 发送音频数据到 EchoKit
            if let Err(e) = self.echokit_client.send_audio_data(
                session.session_id.clone(),
                device_id.to_string(),
                processed_audio.clone(),
                session.input_format,
                false, // 不是最终的音频块
            ).await {
                error!("Failed to send audio to EchoKit: {}", e);
            }

            debug!("Processed {} bytes of audio from device {}", processed_audio.len(), device_id);
        } else {
            warn!("No active session found for device: {}", device_id);
        }

        Ok(())
    }

    // 处理来自 EchoKit 的音频响应
    pub async fn process_echokit_audio(
        &self,
        device_id: &str,
        audio_data: Vec<u8>,
        format: AudioFormat,
    ) -> Result<()> {
        let sessions = self.device_sessions.read().await;

        if let Some(session) = sessions.get(device_id) {
            // 转换音频格式为设备支持的格式
            let output_audio = self.convert_audio_format(
                audio_data,
                format,
                session.output_format,
                session.sample_rate,
                session.channels,
            ).await?;

            // 发送音频数据到设备
            if let Err(e) = self.output_sender.send((device_id.to_string(), output_audio.clone())) {
                error!("Failed to send audio to device {}: {}", device_id, e);
            }

            info!("Sent {} bytes of audio to device {}", output_audio.len(), device_id);
        } else {
            warn!("No active session found for device: {}", device_id);
        }

        Ok(())
    }

    // 转换音频格式
    async fn convert_audio_format(
        &self,
        input_data: Vec<u8>,
        input_format: AudioFormat,
        output_format: AudioFormat,
        sample_rate: u32,
        channels: u8,
    ) -> Result<Vec<u8>> {
        // 如果格式相同，直接返回
        if input_format == output_format {
            return Ok(input_data);
        }

        match (input_format, output_format) {
            (AudioFormat::PCM16, AudioFormat::WAV) => {
                self.pcm16_to_wav(input_data, sample_rate, channels).await
            }
            (AudioFormat::WAV, AudioFormat::PCM16) => {
                self.wav_to_pcm16(input_data).await
            }
            (AudioFormat::PCM16, AudioFormat::Opus) => {
                self.pcm16_to_opus(input_data, sample_rate, channels).await
            }
            (AudioFormat::Opus, AudioFormat::PCM16) => {
                self.opus_to_pcm16(input_data, sample_rate, channels).await
            }
            _ => {
                warn!("Unsupported audio format conversion: {:?} -> {:?}", input_format, output_format);
                Ok(input_data) // 返回原始数据作为降级
            }
        }
    }

    // PCM16 转 WAV
    async fn pcm16_to_wav(&self, pcm_data: Vec<u8>, sample_rate: u32, channels: u8) -> Result<Vec<u8>> {
        let mut wav_data = Vec::new();

        // WAV 文件头
        let data_size = pcm_data.len();
        let file_size = 36 + data_size;

        // RIFF header
        wav_data.extend_from_slice(b"RIFF");
        wav_data.write_u32::<LittleEndian>(file_size as u32)?;
        wav_data.extend_from_slice(b"WAVE");

        // fmt chunk
        wav_data.extend_from_slice(b"fmt ");
        wav_data.write_u32::<LittleEndian>(16)?; // fmt chunk size
        wav_data.write_u16::<LittleEndian>(1)?;  // PCM format
        wav_data.write_u16::<LittleEndian>(channels as u16)?;
        wav_data.write_u32::<LittleEndian>(sample_rate)?;
        wav_data.write_u32::<LittleEndian>(sample_rate * channels as u32 * 2)?; // byte rate
        wav_data.write_u16::<LittleEndian>(channels as u16 * 2)?; // block align
        wav_data.write_u16::<LittleEndian>(16)?; // bits per sample

        // data chunk
        wav_data.extend_from_slice(b"data");
        wav_data.write_u32::<LittleEndian>(data_size as u32)?;
        wav_data.extend_from_slice(&pcm_data);

        Ok(wav_data)
    }

    // WAV 转 PCM16
    async fn wav_to_pcm16(&self, wav_data: Vec<u8>) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(wav_data);

        // 跳过 RIFF header
        cursor.read_exact(&mut [0u8; 12])?;

        // 查找 data chunk
        let mut chunk_id = [0u8; 4];
        let mut chunk_size: u32;

        loop {
            cursor.read_exact(&mut chunk_id)?;
            chunk_size = cursor.read_u32::<LittleEndian>()?;

            if &chunk_id == b"data" {
                break;
            }

            // 跳过非 data chunk
            cursor.set_position(cursor.position() + chunk_size as u64);
        }

        // 读取 PCM 数据
        let mut pcm_data = vec![0u8; chunk_size as usize];
        cursor.read_exact(&mut pcm_data)?;

        Ok(pcm_data)
    }

    // PCM16 转 Opus (简化实现)
    async fn pcm16_to_opus(&self, _pcm_data: Vec<u8>, _sample_rate: u32, _channels: u8) -> Result<Vec<u8>> {
        // TODO: 实现 Opus 编码
        // 这里需要 Opus 库，当前返回原始数据作为占位符
        warn!("Opus encoding not implemented, returning raw data");
        Ok(_pcm_data)
    }

    // Opus 转 PCM16 (简化实现)
    async fn opus_to_pcm16(&self, _opus_data: Vec<u8>, _sample_rate: u32, _channels: u8) -> Result<Vec<u8>> {
        // TODO: 实现 Opus 解码
        // 这里需要 Opus 库，当前返回原始数据作为占位符
        warn!("Opus decoding not implemented, returning raw data");
        Ok(_opus_data)
    }

    // 检查会话超时
    pub async fn check_session_timeouts(&self, timeout_seconds: i64) -> Result<()> {
        let now = now_utc();
        let mut sessions_to_end = Vec::new();

        {
            let sessions = self.device_sessions.read().await;
            for (device_id, session) in sessions.iter() {
                let duration = now.signed_duration_since(session.last_audio_time);
                if duration.num_seconds() > timeout_seconds {
                    sessions_to_end.push(device_id.clone());
                }
            }
        }

        // 结束超时的会话
        for device_id in sessions_to_end {
            warn!("Ending session for device {} due to timeout", device_id);
            if let Err(e) = self.end_session(&device_id, "timeout").await {
                error!("Failed to end timeout session for {}: {}", device_id, e);
            }
        }

        Ok(())
    }

    // 获取活跃会话数量
    pub async fn get_active_sessions_count(&self) -> usize {
        self.device_sessions.read().await.len()
    }

    // 获取设备会话信息
    pub async fn get_session_info(&self, device_id: &str) -> Option<DeviceAudioSession> {
        self.device_sessions.read().await.get(device_id).cloned()
    }
}

// 音频缓冲区管理器
pub struct AudioBuffer {
    chunks: Vec<AudioChunk>,
    max_duration_seconds: f32,
    sample_rate: u32,
}

impl AudioBuffer {
    pub fn new(max_duration_seconds: f32, sample_rate: u32) -> Self {
        Self {
            chunks: Vec::new(),
            max_duration_seconds,
            sample_rate,
        }
    }

    // 添加音频块
    pub fn add_chunk(&mut self, chunk: AudioChunk) {
        self.chunks.push(chunk);
        self.remove_old_chunks();
    }

    // 获取合并的音频数据
    pub fn get_merged_audio(&self) -> Vec<u8> {
        let mut audio_data = Vec::new();
        for chunk in &self.chunks {
            audio_data.extend_from_slice(&chunk.data);
        }
        audio_data
    }

    // 清除缓冲区
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    // 获取缓冲区时长（秒）
    pub fn get_duration_seconds(&self) -> f32 {
        let total_samples = self.chunks.iter()
            .map(|chunk| chunk.data.len() / 2) // 16-bit samples = 2 bytes
            .sum::<usize>();
        total_samples as f32 / self.sample_rate as f32
    }

    // 移除过旧的音频块
    fn remove_old_chunks(&mut self) {
        let max_duration_samples = (self.max_duration_seconds * self.sample_rate as f32) as usize;
        let mut total_samples = 0;

        // 从最新的块开始计算
        for chunk in self.chunks.iter().rev() {
            total_samples += chunk.data.len() / 2;
        }

        // 如果超过最大时长，移除最旧的块
        while total_samples > max_duration_samples && !self.chunks.is_empty() {
            if let Some(oldest_chunk) = self.chunks.first() {
                total_samples -= oldest_chunk.data.len() / 2;
                self.chunks.remove(0);
            } else {
                break;
            }
        }
    }
}

// 音频格式检测器
pub struct AudioFormatDetector;

impl AudioFormatDetector {
    // 根据文件头检测音频格式
    pub fn detect_format(data: &[u8]) -> AudioFormat {
        if data.len() < 12 {
            return AudioFormat::PCM16; // 默认
        }

        // 检测 WAV
        if &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
            return AudioFormat::WAV;
        }

        // 检测 Opus (Ogg 封装)
        if &data[0..4] == b"OggS" {
            return AudioFormat::Opus;
        }

        // 检测 MP3
        if data.len() >= 3 &&
           ((data[0] == 0xFF && (data[1] & 0xE0) == 0xE0) || // MPEG 1/2/2.5 Layer 3
            (data[0] == 0x49 && data[1] == 0x44 && data[2] == 0x33)) { // ID3 tag
            return AudioFormat::MP3;
        }

        // 默认为 PCM16
        AudioFormat::PCM16
    }

    // 验证音频数据完整性
    pub fn validate_audio_data(data: &[u8], format: &AudioFormat) -> bool {
        match format {
            AudioFormat::WAV => {
                data.len() >= 44 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE"
            }
            AudioFormat::Opus => {
                data.len() >= 4 && &data[0..4] == b"OggS"
            }
            AudioFormat::MP3 => {
                data.len() >= 3 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0
            }
            AudioFormat::PCM16 => {
                // PCM16 没有文件头，只能检查长度是否为偶数
                data.len() % 2 == 0
            }
        }
    }
}
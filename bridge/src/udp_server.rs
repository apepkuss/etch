use anyhow::{Context, Result};
use echo_shared::{AudioChunk, AudioFormat, AudioProcessor, now_utc};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

// UDP 音频服务器
pub struct UdpAudioServer {
    socket: Arc<UdpSocket>,
    audio_processor: Arc<AudioProcessor>,
    device_registry: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DeviceInfo>>>,
}

// 设备信息
#[derive(Debug, Clone)]
struct DeviceInfo {
    device_id: String,
    address: SocketAddr,
    last_seen: chrono::DateTime<chrono::Utc>,
    audio_format: AudioFormat,
    sample_rate: u32,
    channels: u8,
    sequence_number: u32,
}

// UDP 数据包格式
#[derive(Debug)]
struct UdpAudioPacket {
    device_id: String,
    sequence_number: u32,
    timestamp: u64,
    audio_data: Vec<u8>,
    flags: u8, // bit 0: is_final, bit 1: is_silence
}

impl UdpAudioServer {
    pub fn new(
        bind_address: &str,
        audio_processor: Arc<AudioProcessor>,
    ) -> Result<Self> {
        let socket = UdpSocket::bind(bind_address)
            .with_context(|| format!("Failed to bind to UDP address: {}", bind_address))?;

        info!("UDP Audio Server listening on: {}", bind_address);

        Ok(Self {
            socket: Arc::new(socket),
            audio_processor,
            device_registry: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        })
    }

    // 启动 UDP 服务器
    pub async fn start(&self) -> Result<()> {
        let socket = self.socket.clone();
        let audio_processor = self.audio_processor.clone();
        let device_registry = self.device_registry.clone();

        info!("Starting UDP Audio Server...");

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096]; // 4KB 缓冲区

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let packet_data = buf[..len].to_vec();

                        if let Err(e) = Self::handle_udp_packet(
                            packet_data,
                            addr,
                            audio_processor.clone(),
                            device_registry.clone(),
                        ).await {
                            error!("Error handling UDP packet: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("UDP receive error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });

        // 启动设备心跳检查任务
        self.start_device_heartbeat_check().await?;

        Ok(())
    }

    // 处理 UDP 数据包
    async fn handle_udp_packet(
        packet_data: Vec<u8>,
        addr: SocketAddr,
        audio_processor: Arc<AudioProcessor>,
        device_registry: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DeviceInfo>>>,
    ) -> Result<()> {
        if packet_data.len() < 16 {
            warn!("Received too small UDP packet: {} bytes", packet_data.len());
            return Ok(());
        }

        // 解析 UDP 数据包
        let packet = Self::parse_udp_packet(packet_data)?;
        let device_id = packet.device_id.clone();

        debug!("Received UDP packet from device: {}, sequence: {}, size: {} bytes",
               device_id, packet.sequence_number, packet.audio_data.len());

        // 更新设备信息
        Self::update_device_info(
            device_registry.clone(),
            device_id.clone(),
            addr,
            packet.sequence_number,
        ).await;

        // 检查设备是否已注册且有活跃会话
        let device_info = {
            let registry = device_registry.read().await;
            registry.get(&device_id).cloned()
        };

        if let Some(device_info) = device_info {
            // 创建音频块
            let audio_chunk = AudioChunk {
                device_id: device_id.clone(),
                sequence_number: packet.sequence_number,
                data: packet.audio_data.clone(),
                timestamp: now_utc(),
            };

            // 处理音频数据
            if let Err(e) = audio_processor.process_device_audio(
                &device_id,
                packet.audio_data,
                device_info.audio_format,
            ).await {
                error!("Failed to process audio from device {}: {}", device_id, e);
            }

            // 如果是最终数据包，处理会话结束逻辑
            if (packet.flags & 0x01) != 0 {
                debug!("Received final audio packet from device: {}", device_id);
                // 这里可以触发音频处理完成逻辑
            }
        } else {
            warn!("Received audio from unregistered device: {}", device_id);
        }

        Ok(())
    }

    // 解析 UDP 数据包
    fn parse_udp_packet(data: Vec<u8>) -> Result<UdpAudioPacket> {
        let mut cursor = Cursor::new(data);

        // 读取设备 ID 长度和 ID
        let device_id_len = cursor.read_u8()? as usize;
        if device_id_len > 64 || cursor.position() as usize + device_id_len > data.len() {
            return Err(anyhow::anyhow!("Invalid device ID length"));
        }

        let mut device_id_bytes = vec![0u8; device_id_len];
        cursor.read_exact(&mut device_id_bytes)?;
        let device_id = String::from_utf8(device_id_bytes)
            .with_context(|| "Invalid device ID (not UTF-8)")?;

        // 读取序列号
        let sequence_number = cursor.read_u32::<LittleEndian>()?;

        // 读取时间戳
        let timestamp = cursor.read_u64::<LittleEndian>()?;

        // 读取标志位
        let flags = cursor.read_u8()?;

        // 读取音频数据长度和数据
        let audio_data_len = cursor.read_u16::<LittleEndian>()? as usize;
        let remaining_bytes = cursor.position() as usize;
        if remaining_bytes + audio_data_len != data.len() {
            return Err(anyhow::anyhow!("Audio data length mismatch"));
        }

        let mut audio_data = vec![0u8; audio_data_len];
        cursor.read_exact(&mut audio_data)?;

        Ok(UdpAudioPacket {
            device_id,
            sequence_number,
            timestamp,
            audio_data,
            flags,
        })
    }

    // 更新设备信息
    async fn update_device_info(
        device_registry: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DeviceInfo>>>,
        device_id: String,
        address: SocketAddr,
        sequence_number: u32,
    ) {
        let mut registry = device_registry.write().await;

        if let Some(device_info) = registry.get_mut(&device_id) {
            device_info.last_seen = now_utc();
            device_info.address = address;
            device_info.sequence_number = sequence_number;
        } else {
            // 新设备，添加默认配置
            let device_info = DeviceInfo {
                device_id: device_id.clone(),
                address,
                last_seen: now_utc(),
                audio_format: AudioFormat::PCM16,
                sample_rate: 16000,
                channels: 1,
                sequence_number,
            };
            registry.insert(device_id, device_info);
            info!("Registered new device: {}", device_id);
        }
    }

    // 启动设备心跳检查
    async fn start_device_heartbeat_check(&self) -> Result<()> {
        let device_registry = self.device_registry.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                let now = now_utc();
                let mut devices_to_remove = Vec::new();

                {
                    let registry = device_registry.read().await;
                    for (device_id, device_info) in registry.iter() {
                        let duration = now.signed_duration_since(device_info.last_seen);
                        if duration.num_seconds() > 60 { // 60秒无心跳认为设备离线
                            devices_to_remove.push(device_id.clone());
                        }
                    }
                }

                // 移除离线设备
                if !devices_to_remove.is_empty() {
                    let mut registry = device_registry.write().await;
                    for device_id in &devices_to_remove {
                        registry.remove(device_id);
                        warn!("Device {} removed due to heartbeat timeout", device_id);
                    }
                }
            }
        });

        Ok(())
    }

    // 注册设备
    pub async fn register_device(
        &self,
        device_id: String,
        audio_format: AudioFormat,
        sample_rate: u32,
        channels: u8,
    ) -> Result<()> {
        let mut registry = self.device_registry.write().await;

        let device_info = DeviceInfo {
            device_id: device_id.clone(),
            address: "0.0.0.0:0".parse().unwrap(), // 占位地址
            last_seen: now_utc(),
            audio_format,
            sample_rate,
            channels,
            sequence_number: 0,
        };

        registry.insert(device_id.clone(), device_info);
        info!("Registered device: {} with format: {:?}, rate: {}, channels: {}",
              device_id, audio_format, sample_rate, channels);

        Ok(())
    }

    // 注销设备
    pub async fn unregister_device(&self, device_id: &str) -> Result<()> {
        let mut registry = self.device_registry.write().await;
        if registry.remove(device_id).is_some() {
            info!("Unregistered device: {}", device_id);
            Ok(())
        } else {
            warn!("Device {} not found for unregistration", device_id);
            Err(anyhow::anyhow!("Device not found"))
        }
    }

    // 获取设备列表
    pub async fn get_registered_devices(&self) -> Vec<String> {
        self.device_registry.read().await.keys().cloned().collect()
    }

    // 获取设备信息
    pub async fn get_device_info(&self, device_id: &str) -> Option<DeviceInfo> {
        self.device_registry.read().await.get(device_id).cloned()
    }

    // 发送数据到设备
    pub async fn send_to_device(&self, device_id: &str, data: Vec<u8>) -> Result<()> {
        let registry = self.device_registry.read().await;

        if let Some(device_info) = registry.get(device_id) {
            self.socket.send_to(&data, device_info.address).await
                .with_context(|| format!("Failed to send data to device: {}", device_id))?;

            debug!("Sent {} bytes to device: {}", data.len(), device_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Device {} not found", device_id))
        }
    }

    // 广播数据到所有设备
    pub async fn broadcast_to_devices(&self, data: Vec<u8>) -> Result<usize> {
        let registry = self.device_registry.read().await;
        let mut sent_count = 0;

        for (device_id, device_info) in registry.iter() {
            if let Err(e) = self.socket.send_to(&data, device_info.address).await {
                error!("Failed to send broadcast to device {}: {}", device_id, e);
            } else {
                sent_count += 1;
            }
        }

        debug!("Broadcasted {} bytes to {} devices", data.len(), sent_count);
        Ok(sent_count)
    }

    // 获取服务器统计信息
    pub async fn get_stats(&self) -> UdpServerStats {
        let registry = self.device_registry.read().await;
        let online_devices = registry.len();

        UdpServerStats {
            online_devices,
            bind_address: self.socket.local_addr().unwrap().to_string(),
            uptime_seconds: 0, // TODO: 实现运行时间统计
        }
    }
}

// UDP 服务器统计信息
#[derive(Debug, Clone)]
pub struct UdpServerStats {
    pub online_devices: usize,
    pub bind_address: String,
    pub uptime_seconds: u64,
}

// 创建 UDP 数据包的工具函数
pub struct UdpPacketBuilder;

impl UdpPacketBuilder {
    // 创建音频数据包
    pub fn create_audio_packet(
        device_id: &str,
        sequence_number: u32,
        timestamp: u64,
        audio_data: Vec<u8>,
        is_final: bool,
    ) -> Result<Vec<u8>> {
        let mut packet = Vec::new();

        // 设备 ID
        let device_id_bytes = device_id.as_bytes();
        if device_id_bytes.len() > 255 {
            return Err(anyhow::anyhow!("Device ID too long"));
        }
        packet.push(device_id_bytes.len() as u8);
        packet.extend_from_slice(device_id_bytes);

        // 序列号
        packet.extend_from_slice(&sequence_number.to_le_bytes());

        // 时间戳
        packet.extend_from_slice(&timestamp.to_le_bytes());

        // 标志位
        let flags = if is_final { 0x01 } else { 0x00 };
        packet.push(flags);

        // 音频数据长度
        if audio_data.len() > 65535 {
            return Err(anyhow::anyhow!("Audio data too large"));
        }
        packet.extend_from_slice(&(audio_data.len() as u16).to_le_bytes());

        // 音频数据
        packet.extend_from_slice(&audio_data);

        Ok(packet)
    }

    // 创建控制数据包
    pub fn create_control_packet(
        device_id: &str,
        command: &str,
        parameters: &std::collections::HashMap<String, String>,
    ) -> Result<Vec<u8>> {
        let mut packet = Vec::new();

        // 设备 ID
        let device_id_bytes = device_id.as_bytes();
        if device_id_bytes.len() > 255 {
            return Err(anyhow::anyhow!("Device ID too long"));
        }
        packet.push(device_id_bytes.len() as u8);
        packet.extend_from_slice(device_id_bytes);

        // 命令
        let command_bytes = command.as_bytes();
        if command_bytes.len() > 255 {
            return Err(anyhow::anyhow!("Command too long"));
        }
        packet.push(command_bytes.len() as u8);
        packet.extend_from_slice(command_bytes);

        // 参数数量
        packet.push(parameters.len() as u8);

        // 参数
        for (key, value) in parameters {
            // Key
            let key_bytes = key.as_bytes();
            if key_bytes.len() > 255 {
                return Err(anyhow::anyhow!("Parameter key too long"));
            }
            packet.push(key_bytes.len() as u8);
            packet.extend_from_slice(key_bytes);

            // Value
            let value_bytes = value.as_bytes();
            if value_bytes.len() > 255 {
                return Err(anyhow::anyhow!("Parameter value too long"));
            }
            packet.push(value_bytes.len() as u8);
            packet.extend_from_slice(value_bytes);
        }

        Ok(packet)
    }
}
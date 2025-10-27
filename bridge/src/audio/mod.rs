use echo_shared::AppConfig;
use tokio::net::UdpSocket;
use std::sync::Arc;
use tracing::{info, warn, error};

// 音频数据包结构
#[derive(Debug, Clone)]
pub struct AudioPacket {
    pub device_id: String,
    pub sequence: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub audio_data: Vec<u8>,
}

pub async fn handle_audio_stream(
    socket: Arc<UdpSocket>,
    config: AppConfig,
) -> anyhow::Result<()> {
    info!("Starting audio stream handler");

    let mut buf = [0u8; 1024]; // 1KB buffer for audio packets

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                let data = &buf[..len];

                // 尝试解析音频数据包
                match parse_audio_packet(data) {
                    Ok(packet) => {
                        info!("Received audio packet from {}: {:?}", addr, packet);

                        // 处理音频数据
                        if let Err(e) = process_audio_packet(packet, &config).await {
                            error!("Error processing audio packet: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse audio packet from {}: {}", addr, e);
                    }
                }
            }
            Err(e) => {
                error!("Error receiving UDP packet: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

async fn process_audio_packet(
    packet: AudioPacket,
    _config: &AppConfig,
) -> anyhow::Result<()> {
    info!("Processing audio packet from device: {}", packet.device_id);

    // 这里可以实现音频处理逻辑：
    // 1. 音频解码/编码
    // 2. 语音识别
    // 3. 发送到其他服务

    // 模拟处理
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    Ok(())
}

fn parse_audio_packet(data: &[u8]) -> anyhow::Result<AudioPacket> {
    if data.len() < 16 {
        return Err(anyhow::anyhow!("Packet too short"));
    }

    // 简单的协议解析：
    // 第1字节：device_id长度
    // 接下来N字节：device_id (字符串)
    // 接下来4字节：sequence
    // 接下来8字节：timestamp
    // 剩余：audio_data

    let device_id_len = data[0] as usize;
    if data.len() < 1 + device_id_len + 12 {
        return Err(anyhow::anyhow!("Invalid packet format"));
    }

    // 读取device_id
    let device_id = String::from_utf8(data[1..1 + device_id_len].to_vec())?;

    // 读取sequence
    let sequence_start = 1 + device_id_len;
    let sequence_bytes = &data[sequence_start..sequence_start + 4];
    let sequence = u32::from_le_bytes(sequence_bytes.try_into()?);

    // 读取timestamp
    let timestamp_start = sequence_start + 4;
    let timestamp_bytes = &data[timestamp_start..timestamp_start + 8];
    let timestamp_nanos = u64::from_le_bytes(timestamp_bytes.try_into()?);
    let timestamp = chrono::DateTime::from_timestamp(
        (timestamp_nanos / 1_000_000_000) as i64,
        (timestamp_nanos % 1_000_000_000) as u32,
    ).ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;

    // 读取audio_data
    let audio_data = data[timestamp_start + 8..].to_vec();

    Ok(AudioPacket {
        device_id,
        sequence,
        timestamp,
        audio_data,
    })
}

// 创建示例音频数据包
pub fn create_audio_packet(device_id: &str, sequence: u32, audio_data: Vec<u8>) -> Vec<u8> {
    let mut packet = Vec::new();

    // device_id长度
    packet.push(device_id.len() as u8);

    // device_id
    packet.extend_from_slice(device_id.as_bytes());

    // sequence
    packet.extend_from_slice(&sequence.to_le_bytes());

    // timestamp
    let timestamp_nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    packet.extend_from_slice(&timestamp_nanos.to_le_bytes());

    // audio_data
    packet.extend_from_slice(&audio_data);

    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_packet_parsing() {
        let device_id = "test_device";
        let sequence = 12345u32;
        let audio_data = vec![1, 2, 3, 4, 5];

        let packet_data = create_audio_packet(device_id, sequence, audio_data.clone());
        let parsed_packet = parse_audio_packet(&packet_data).unwrap();

        assert_eq!(parsed_packet.device_id, device_id);
        assert_eq!(parsed_packet.sequence, sequence);
        assert_eq!(parsed_packet.audio_data, audio_data);
    }
}
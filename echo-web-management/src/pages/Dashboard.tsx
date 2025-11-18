import React, { useEffect } from 'react';
import { Card, Row, Col, Statistic, Button } from 'antd';
import {
  AudioOutlined,
  UserOutlined,
  HistoryOutlined,
  WifiOutlined,
  PlayCircleOutlined
} from '@ant-design/icons';
import { useDeviceStore } from '../stores/useDeviceStore';
import { useSessionStore } from '../stores/useSessionStore';
import { DeviceStatus, DeviceType } from '../types';

export const Dashboard: React.FC = () => {
  const { devices, stats, loading, fetchDevices, fetchDeviceStats } = useDeviceStore();
  const { activeSessions, stats: sessionStats, fetchSessionStats } = useSessionStore();

  useEffect(() => {
    fetchDevices();
    fetchDeviceStats();
    fetchSessionStats();
  }, [fetchDevices, fetchDeviceStats, fetchSessionStats]);

  return (
    <div style={{ padding: 24 }}>
      {/* 页面标题 */}
      <div style={{ marginBottom: 24 }}>
        <h2>仪表板</h2>
        <p style={{ color: '#8c8c8c' }}>智能音箱设备管理概览</p>
      </div>

      {/* 统计卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="设备总数"
              value={stats.total}
              prefix={<AudioOutlined />}
              loading={loading}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="在线设备"
              value={stats.online}
              valueStyle={{ color: '#3f8600' }}
              prefix={<WifiOutlined />}
              loading={loading}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="活跃会话"
              value={sessionStats.active}
              valueStyle={{ color: '#1890ff' }}
              prefix={<PlayCircleOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="总会话数"
              value={sessionStats.total}
              prefix={<HistoryOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* 快速操作 */}
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card title="快速操作">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <Button type="primary" icon={<AudioOutlined />} size="large" style={{ textAlign: 'left' }}>
                添加新设备
              </Button>
              <Button icon={<WifiOutlined />} size="large" style={{ textAlign: 'left' }}>
                查看设备状态
              </Button>
              <Button icon={<HistoryOutlined />} size="large" style={{ textAlign: 'left' }}>
                查看会话记录
              </Button>
            </div>
          </Card>
        </Col>

        <Col xs={24} lg={12}>
          <Card title="系统状态">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>WebSocket连接</span>
                <span style={{ color: '#52c41a' }}>● 已连接</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>数据同步</span>
                <span style={{ color: '#52c41a' }}>● 正常</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>系统状态</span>
                <span style={{ color: '#52c41a' }}>● 运行中</span>
              </div>
            </div>
          </Card>
        </Col>
      </Row>

      {/* 最近设备 */}
      <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
        <Col xs={24}>
          <Card title="设备概览">
            <Row gutter={[16, 16]}>
              {devices.slice(0, 4).map((device) => (
                <Col xs={24} sm={12} lg={6} key={device.id}>
                  <Card size="small" style={{ textAlign: 'center' }}>
                    <div style={{ fontSize: 24, marginBottom: 8 }}>
                      {device.device_type === DeviceType.Speaker ? '🔊' : '🎛️'}
                    </div>
                    <div style={{ fontWeight: 500, marginBottom: 4 }}>
                      {device.name}
                    </div>
                    <div style={{ fontSize: 12, color: '#8c8c8c', marginBottom: 8 }}>
                      {device.location}
                    </div>
                    <div style={{
                      display: 'inline-block',
                      padding: '2px 8px',
                      borderRadius: 4,
                      fontSize: 12,
                      backgroundColor: device.status === DeviceStatus.Online ? '#f6ffed' : '#fff1f0',
                      color: device.status === DeviceStatus.Online ? '#52c41a' : '#ff4d4f',
                      border: `1px solid ${device.status === DeviceStatus.Online ? '#b7eb8f' : '#ffccc7'}`
                    }}>
                      {device.status === DeviceStatus.Online ? '在线' : '离线'}
                    </div>
                  </Card>
                </Col>
              ))}
            </Row>
          </Card>
        </Col>
      </Row>
    </div>
  );
};
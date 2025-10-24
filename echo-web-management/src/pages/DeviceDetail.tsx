import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Card,
  Row,
  Col,
  Tag,
  Button,
  Descriptions,
  Progress,
  Space,
  Tabs,
  Form,
  Input,
  Select,
  Slider,
  message,
  Spin,
  Alert,
  Timeline,
  Statistic
} from 'antd';
import {
  ArrowLeftOutlined,
  SettingOutlined,
  ReloadOutlined,
  PlayCircleOutlined,
  PauseCircleOutlined,
  WifiOutlined,
  BatteryOutlined,
  AudioOutlined,
  DesktopOutlined,
  ControlOutlined,
  HistoryOutlined
} from '@ant-design/icons';
import { useDeviceStore } from '../stores/useDeviceStore';
import { useSessionStore } from '../stores/useSessionStore';
import { Device, DeviceStatus, DeviceType, Session } from '../types';
import { useWebSocket } from '../hooks/useWebSocket';

const { TabPane } = Tabs;

export const DeviceDetail: React.FC = () => {
  const { deviceId } = useParams<{ deviceId: string }>();
  const navigate = useNavigate();
  const { devices, selectedDevice, selectDevice, updateDeviceConfig, restartDevice } = useDeviceStore();
  const { sessions, fetchSessions } = useSessionStore();
  const { sendDeviceCommand, isConnected } = useWebSocket();

  const [form] = Form.useForm();
  const [configLoading, setConfigLoading] = useState(false);

  // 查找设备
  const device = deviceId ? devices.find(d => d.id === deviceId) : null;

  useEffect(() => {
    if (device) {
      selectDevice(device);
      fetchSessions(device.id);
      form.setFieldsValue({
        volume: device.volume,
        location: device.location
      });
    }
  }, [device, selectDevice, fetchSessions, form]);

  // 获取状态颜色
  const getStatusColor = (status: DeviceStatus) => {
    switch (status) {
      case DeviceStatus.ONLINE: return 'success';
      case DeviceStatus.OFFLINE: return 'default';
      case DeviceStatus.ERROR: return 'error';
      case DeviceStatus.MAINTENANCE: return 'warning';
      default: return 'default';
    }
  };

  // 获取状态文本
  const getStatusText = (status: DeviceStatus) => {
    switch (status) {
      case DeviceStatus.ONLINE: return '在线';
      case DeviceStatus.OFFLINE: return '离线';
      case DeviceStatus.ERROR: return '故障';
      case DeviceStatus.MAINTENANCE: return '维护中';
      default: return '未知';
    }
  };

  // 获取设备类型图标
  const getDeviceTypeIcon = (type: DeviceType) => {
    switch (type) {
      case DeviceType.SPEAKER: return <AudioOutlined />;
      case DeviceType.DISPLAY: return <DesktopOutlined />;
      case DeviceType.HUB: return <ControlOutlined />;
      default: return <AudioOutlined />;
    }
  };

  // 处理配置更新
  const handleConfigUpdate = async (values: any) => {
    if (!device) return;

    setConfigLoading(true);
    try {
      await updateDeviceConfig(device.id, values);
      message.success('设备配置更新成功');
    } catch (error) {
      message.error('配置更新失败');
    } finally {
      setConfigLoading(false);
    }
  };

  // 处理重启设备
  const handleRestartDevice = async () => {
    if (!device) return;

    try {
      await restartDevice(device.id);
      message.success('重启指令已发送');
    } catch (error) {
      message.error('重启失败');
    }
  };

  // 发送语音测试命令
  const handleVoiceTest = () => {
    if (!device) return;
    sendDeviceCommand(device.id, 'voice_test');
    message.info('语音测试指令已发送');
  };

  // 获取设备会话
  const deviceSessions = sessions.filter(session => session.deviceId === deviceId);

  if (!device) {
    return (
      <div style={{ padding: 24, textAlign: 'center' }}>
        <Spin size="large" />
        <p style={{ marginTop: 16 }}>加载设备信息中...</p>
      </div>
    );
  }

  return (
    <div style={{ padding: 24 }}>
      {/* 页面头部 */}
      <Card style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Space>
            <Button
              icon={<ArrowLeftOutlined />}
              onClick={() => navigate('/devices')}
            >
              返回设备列表
            </Button>
            <div>
              <h2 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: 8 }}>
                {getDeviceTypeIcon(device.type)}
                {device.name}
              </h2>
              <p style={{ margin: 0, color: '#8c8c8c' }}>
                {device.location} • ID: {device.id}
              </p>
            </div>
          </Space>
          <Space>
            <Tag color={getStatusColor(device.status)}>
              {getStatusText(device.status)}
            </Tag>
            <Button
              icon={<ReloadOutlined />}
              onClick={handleRestartDevice}
              disabled={device.status === DeviceStatus.MAINTENANCE}
            >
              重启设备
            </Button>
            <Button
              icon={<PlayCircleOutlined />}
              onClick={handleVoiceTest}
              disabled={!isConnected || device.status !== DeviceStatus.ONLINE}
            >
              语音测试
            </Button>
          </Space>
        </div>
      </Card>

      {/* 连接状态提示 */}
      {!isConnected && (
        <Alert
          message="WebSocket未连接"
          description="实时功能可能不可用，请检查网络连接。"
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 设备详情标签页 */}
      <Tabs defaultActiveKey="overview">
        {/* 设备概览 */}
        <TabPane tab="设备概览" key="overview">
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <Card title="基本信息">
                <Descriptions column={1}>
                  <Descriptions.Item label="设备ID">{device.id}</Descriptions.Item>
                  <Descriptions.Item label="设备名称">{device.name}</Descriptions.Item>
                  <Descriptions.Item label="设备类型">
                    {device.type === DeviceType.SPEAKER ? '智能音箱' :
                     device.type === DeviceType.DISPLAY ? '智能显示屏' : '中控设备'}
                  </Descriptions.Item>
                  <Descriptions.Item label="设备位置">{device.location}</Descriptions.Item>
                  <Descriptions.Item label="固件版本">{device.firmwareVersion}</Descriptions.Item>
                  <Descriptions.Item label="设备所有者">{device.owner}</Descriptions.Item>
                </Descriptions>
              </Card>
            </Col>

            <Col xs={24} md={12}>
              <Card title="实时状态">
                <Space direction="vertical" style={{ width: '100%' }}>
                  <div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
                      <span>电池电量</span>
                      {device.batteryLevel && (
                        <span>
                          <BatteryOutlined /> {device.batteryLevel}%
                        </span>
                      )}
                    </div>
                    {device.batteryLevel && (
                      <Progress
                        percent={device.batteryLevel}
                        strokeColor={device.batteryLevel > 20 ? '#52c41a' : '#ff4d4f'}
                        showInfo={false}
                      />
                    )}
                  </div>

                  <div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
                      <span>音量</span>
                      <span>{device.volume}%</span>
                    </div>
                    <Progress percent={device.volume} showInfo={false} />
                  </div>

                  <Statistic
                    title="最后在线时间"
                    value={new Date(device.lastSeen).toLocaleString()}
                    prefix={<WifiOutlined />}
                  />
                </Space>
              </Card>
            </Col>
          </Row>
        </TabPane>

        {/* 设备配置 */}
        <TabPane tab="设备配置" key="config">
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <Card title="基本配置">
                <Form
                  form={form}
                  layout="vertical"
                  onFinish={handleConfigUpdate}
                >
                  <Form.Item
                    name="volume"
                    label="音量设置"
                  >
                    <Slider
                      min={0}
                      max={100}
                      marks={{
                        0: '静音',
                        50: '50%',
                        100: '最大'
                      }}
                    />
                  </Form.Item>

                  <Form.Item
                    name="location"
                    label="设备位置"
                    rules={[{ required: true, message: '请输入设备位置' }]}
                  >
                    <Input placeholder="请输入设备位置" />
                  </Form.Item>

                  <Form.Item>
                    <Button type="primary" htmlType="submit" loading={configLoading}>
                      保存配置
                    </Button>
                  </Form.Item>
                </Form>
              </Card>
            </Col>

            <Col xs={24} md={12}>
              <Card title="高级配置">
                <Space direction="vertical" style={{ width: '100%' }}>
                  <div>
                    <label>唤醒词设置</label>
                    <Select
                      defaultValue="小智小智"
                      style={{ width: '100%', marginTop: 8 }}
                      options={[
                        { value: '小智小智', label: '小智小智' },
                        { value: 'Echo', label: 'Echo' },
                        { value: '智能助手', label: '智能助手' }
                      ]}
                    />
                  </div>

                  <div>
                    <label>语音设置</label>
                    <Select
                      defaultValue="female"
                      style={{ width: '100%', marginTop: 8 }}
                      options={[
                        { value: 'female', label: '女声' },
                        { value: 'male', label: '男声' }
                      ]}
                    />
                  </div>

                  <div>
                    <label>语言设置</label>
                    <Select
                      defaultValue="zh-CN"
                      style={{ width: '100%', marginTop: 8 }}
                      options={[
                        { value: 'zh-CN', label: '中文（简体）' },
                        { value: 'zh-TW', label: '中文（繁体）' },
                        { value: 'en-US', label: 'English' }
                      ]}
                    />
                  </div>
                </Space>
              </Card>
            </Col>
          </Row>
        </TabPane>

        {/* 会话历史 */}
        <TabPane tab="会话历史" key="sessions">
          <Card
            title="近期会话"
            extra={
              <span>
                共 {deviceSessions.length} 条记录
              </span>
            }
          >
            <Timeline>
              {deviceSessions.map((session) => (
                <Timeline.Item
                  key={session.id}
                  color={session.status === 'active' ? 'green' : 'blue'}
                >
                  <div>
                    <div style={{ fontWeight: 500 }}>
                      {new Date(session.startTime).toLocaleString()}
                    </div>
                    <div style={{ marginTop: 4 }}>
                      <span style={{ color: '#1890ff' }}>用户：</span>
                      "{session.transcription}"
                    </div>
                    <div style={{ marginTop: 4 }}>
                      <span style={{ color: '#52c41a' }}>助手：</span>
                      "{session.response}"
                    </div>
                    {session.duration && (
                      <div style={{ marginTop: 4, fontSize: 12, color: '#8c8c8c' }}>
                        <HistoryOutlined /> 时长: {session.duration}秒
                      </div>
                    )}
                  </div>
                </Timeline.Item>
              ))}
              {deviceSessions.length === 0 && (
                <div style={{ textAlign: 'center', color: '#8c8c8c', padding: 20 }}>
                  暂无会话记录
                </div>
              )}
            </Timeline>
          </Card>
        </TabPane>
      </Tabs>
    </div>
  );
};
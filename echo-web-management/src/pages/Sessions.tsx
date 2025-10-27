import React, { useEffect, useState } from 'react';
import {
  Card,
  Table,
  Tag,
  Space,
  Input,
  Select,
  DatePicker,
  Button,
  Avatar,
  Timeline,
  Modal,
  Descriptions,
  Statistic,
  Row,
  Col,
  message
} from 'antd';
import {
  SearchOutlined,
  ReloadOutlined,
  UserOutlined,
  AudioOutlined,
  DesktopOutlined,
  ControlOutlined,
  ClockCircleOutlined,
  PlayCircleOutlined,
  PauseCircleOutlined,
  StopOutlined
} from '@ant-design/icons';
import { useSessionStore } from '../stores/useSessionStore';
import { useDeviceStore } from '../stores/useDeviceStore';
import { Session, SessionStatus, Device, DeviceType } from '../types';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';

const { RangePicker } = DatePicker;

export const Sessions: React.FC = () => {
  const {
    sessions,
    loading,
    error,
    fetchSessions,
    fetchSessionStats,
    stats,
    completeSession,
    interruptSession
  } = useSessionStore();

  const {
    devices
  } = useDeviceStore();

  const [searchText, setSearchText] = useState('');
  const [filterDevice, setFilterDevice] = useState<string>('all');
  const [filterStatus, setFilterStatus] = useState<SessionStatus | 'all'>('all');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs] | null>(null);
  const [selectedSession, setSelectedSession] = useState<Session | null>(null);
  const [isModalVisible, setIsModalVisible] = useState(false);

  useEffect(() => {
    fetchSessions();
    fetchSessionStats();
  }, [fetchSessions, fetchSessionStats]);

  // 显示错误消息
  useEffect(() => {
    if (error) {
      message.error(error);
    }
  }, [error]);

  // 过滤会话
  const filteredSessions = sessions.filter(session => {
    const transcription = session.transcription || '';
    const response = session.response || '';

    const matchesSearch = transcription.toLowerCase().includes(searchText.toLowerCase()) ||
                         response.toLowerCase().includes(searchText.toLowerCase());
    const matchesDevice = filterDevice === 'all' || session.device_id === filterDevice;
    const matchesStatus = filterStatus === 'all' || session.status === filterStatus;

    let matchesDate = true;
    if (dateRange && dateRange[0] && dateRange[1]) {
      const sessionDate = dayjs(session.start_time);
      matchesDate = sessionDate.isAfter(dateRange[0]) && sessionDate.isBefore(dateRange[1]);
    }

    return matchesSearch && matchesDevice && matchesStatus && matchesDate;
  });

  // 获取设备名称
  const getDeviceName = (deviceId: string) => {
    const device = devices.find(d => d.id === deviceId);
    return device?.name || '未知设备';
  };

  // 获取设备类型图标
  const getDeviceTypeIcon = (deviceId: string) => {
    const device = devices.find(d => d.id === deviceId);
    if (!device) return <AudioOutlined />;

    switch (device.device_type) {
      case DeviceType.Speaker: return <AudioOutlined />;
      case DeviceType.Display: return <DesktopOutlined />;
      case DeviceType.Hub: return <ControlOutlined />;
      default: return <AudioOutlined />;
    }
  };

  // 获取状态颜色
  const getStatusColor = (status: SessionStatus) => {
    switch (status) {
      case SessionStatus.Active: return 'processing';
      case SessionStatus.Completed: return 'success';
      case SessionStatus.Interrupted: return 'error';
      default: return 'default';
    }
  };

  // 获取状态文本
  const getStatusText = (status: SessionStatus) => {
    switch (status) {
      case SessionStatus.Active: return '进行中';
      case SessionStatus.Completed: return '已完成';
      case SessionStatus.Interrupted: return '已中断';
      default: return '未知';
    }
  };

  // 获取状态图标
  const getStatusIcon = (status: SessionStatus) => {
    switch (status) {
      case SessionStatus.Active: return <PlayCircleOutlined />;
      case SessionStatus.Completed: return <StopOutlined />;
      case SessionStatus.Interrupted: return <PauseCircleOutlined />;
      default: return <ClockCircleOutlined />;
    }
  };

  // 格式化时长
  const formatDuration = (duration?: number) => {
    if (!duration) return '-';
    const minutes = Math.floor(duration / 60);
    const seconds = duration % 60;
    return `${minutes}分${seconds}秒`;
  };

  // 刷新会话列表
  const handleRefresh = () => {
    fetchSessions();
    fetchSessionStats();
    message.success('会话列表已刷新');
  };

  // 完成会话
  const handleCompleteSession = async (sessionId: string) => {
    try {
      await completeSession(sessionId, '用户手动完成', '会话已成功完成');
      message.success('会话已完成');
    } catch (error) {
      message.error('完成会话失败');
    }
  };

  // 中断会话
  const handleInterruptSession = async (sessionId: string) => {
    try {
      await interruptSession(sessionId);
      message.success('会话已中断');
    } catch (error) {
      message.error('中断会话失败');
    }
  };

  // 查看会话详情
  const handleViewSession = (session: Session) => {
    setSelectedSession(session);
    setIsModalVisible(true);
  };

  // 表格列定义
  const columns: ColumnsType<Session> = [
    {
      title: '会话ID',
      dataIndex: 'id',
      key: 'id',
      width: 120,
      render: (id: string) => (
        <span style={{ fontFamily: 'monospace', fontSize: '12px' }}>
          {id.slice(0, 8)}...
        </span>
      )
    },
    {
      title: '设备',
      key: 'device',
      render: (_, record) => (
        <Space>
          <Avatar
            icon={getDeviceTypeIcon(record.device_id)}
            size="small"
          />
          <div>
            <div style={{ fontWeight: 'bold' }}>{getDeviceName(record.device_id)}</div>
            <div style={{ color: '#666', fontSize: '12px' }}>{record.device_id}</div>
          </div>
        </Space>
      )
    },
    {
      title: '用户',
      dataIndex: 'user_id',
      key: 'user_id',
      render: (userId: string) => (
        <Space>
          <Avatar icon={<UserOutlined />} size="small" />
          <span>{userId}</span>
        </Space>
      )
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: SessionStatus) => (
        <Tag color={getStatusColor(status)} icon={getStatusIcon(status)}>
          {getStatusText(status)}
        </Tag>
      )
    },
    {
      title: '开始时间',
      dataIndex: 'start_time',
      key: 'start_time',
      render: (time: string) => (
        <Tooltip title={new Date(time).toLocaleString()}>
          {dayjs(time).format('MM-DD HH:mm')}
        </Tooltip>
      )
    },
    {
      title: '结束时间',
      dataIndex: 'end_time',
      key: 'end_time',
      render: (time: string | null) => (
        time ? (
          <Tooltip title={new Date(time).toLocaleString()}>
            {dayjs(time).format('MM-DD HH:mm')}
          </Tooltip>
        ) : '-'
      )
    },
    {
      title: '时长',
      dataIndex: 'duration',
      key: 'duration',
      render: (duration: number | null) => formatDuration(duration || 0)
    },
    {
      title: '语音内容',
      dataIndex: 'transcription',
      key: 'transcription',
      width: 200,
      ellipsis: true,
      render: (text: string) => text || '-'
    },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            type="link"
            size="small"
            onClick={() => handleViewSession(record)}
          >
            详情
          </Button>
          {record.status === SessionStatus.Active && (
            <>
              <Button
                type="link"
                size="small"
                onClick={() => handleCompleteSession(record.id)}
              >
                完成
              </Button>
              <Button
                type="link"
                size="small"
                danger
                onClick={() => handleInterruptSession(record.id)}
              >
                中断
              </Button>
            </>
          )}
        </Space>
      )
    }
  ];

  return (
    <div style={{ padding: '24px' }}>
      {/* 统计卡片 */}
      <Row gutter={16} style={{ marginBottom: '24px' }}>
        <Col span={6}>
          <Card>
            <Statistic
              title="总会话数"
              value={stats.total}
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="活跃会话"
              value={stats.active}
              valueStyle={{ color: '#1890ff' }}
              prefix={<PlayCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="已完成"
              value={stats.completed}
              valueStyle={{ color: '#52c41a' }}
              prefix={<StopOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="已中断"
              value={stats.interrupted}
              valueStyle={{ color: '#ff4d4f' }}
              prefix={<PauseCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* 会话列表 */}
      <Card
        title="会话管理"
        extra={
          <Button
            icon={<ReloadOutlined />}
            onClick={handleRefresh}
            loading={loading}
          >
            刷新
          </Button>
        }
      >
        {/* 搜索和过滤 */}
        <Space style={{ marginBottom: '16px' }}>
          <Input
            placeholder="搜索语音内容或回复"
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 200 }}
          />
          <Select
            placeholder="设备过滤"
            value={filterDevice}
            onChange={setFilterDevice}
            style={{ width: 150 }}
          >
            <Select.Option value="all">全部设备</Select.Option>
            {devices.map(device => (
              <Select.Option key={device.id} value={device.id}>
                {device.name}
              </Select.Option>
            ))}
          </Select>
          <Select
            placeholder="状态过滤"
            value={filterStatus}
            onChange={setFilterStatus}
            style={{ width: 120 }}
          >
            <Select.Option value="all">全部状态</Select.Option>
            <Select.Option value={SessionStatus.Active}>进行中</Select.Option>
            <Select.Option value={SessionStatus.Completed}>已完成</Select.Option>
            <Select.Option value={SessionStatus.Interrupted}>已中断</Select.Option>
          </Select>
          <RangePicker
            value={dateRange}
            onChange={setDateRange}
            format="YYYY-MM-DD"
            placeholder={['开始日期', '结束日期']}
          />
        </Space>

        {/* 会话表格 */}
        <Table
          columns={columns}
          dataSource={filteredSessions}
          rowKey="id"
          loading={loading}
          pagination={{
            total: filteredSessions.length,
            pageSize: 10,
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total, range) => `第 ${range[0]}-${range[1]} 条，共 ${total} 条`
          }}
        />
      </Card>

      {/* 会话详情模态框 */}
      <Modal
        title="会话详情"
        open={isModalVisible}
        onCancel={() => {
          setIsModalVisible(false);
          setSelectedSession(null);
        }}
        footer={null}
        width={600}
      >
        {selectedSession && (
          <Descriptions column={1} bordered>
            <Descriptions.Item label="会话ID">
              <code>{selectedSession.id}</code>
            </Descriptions.Item>
            <Descriptions.Item label="设备">
              <Space>
                {getDeviceTypeIcon(selectedSession.device_id)}
                {getDeviceName(selectedSession.device_id)}
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="用户">
              <Space>
                <Avatar icon={<UserOutlined />} size="small" />
                {selectedSession.user_id}
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="状态">
              <Tag color={getStatusColor(selectedSession.status)} icon={getStatusIcon(selectedSession.status)}>
                {getStatusText(selectedSession.status)}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="开始时间">
              {new Date(selectedSession.start_time).toLocaleString()}
            </Descriptions.Item>
            <Descriptions.Item label="结束时间">
              {selectedSession.end_time ? new Date(selectedSession.end_time).toLocaleString() : '-'}
            </Descriptions.Item>
            <Descriptions.Item label="时长">
              {formatDuration(selectedSession.duration || 0)}
            </Descriptions.Item>
            <Descriptions.Item label="语音内容">
              {selectedSession.transcription || '-'}
            </Descriptions.Item>
            <Descriptions.Item label="回复内容">
              {selectedSession.response || '-'}
            </Descriptions.Item>
          </Descriptions>
        )}
      </Modal>
    </div>
  );
};

export default Sessions;
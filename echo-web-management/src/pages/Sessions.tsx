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
  Col
} from 'antd';
import {
  SearchOutlined,
  ReloadOutlined,
  UserOutlined,
  AudioOutlined,
  DesktopOutlined,
  ControlOutlined,
  ClockCircleOutlined,
  PlayCircleOutlined
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
    fetchSessions,
    stats
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
  }, [fetchSessions]);

  // 过滤会话
  const filteredSessions = sessions.filter(session => {
    const matchesSearch = session.transcription.toLowerCase().includes(searchText.toLowerCase()) ||
                         session.response.toLowerCase().includes(searchText.toLowerCase());
    const matchesDevice = filterDevice === 'all' || session.deviceId === filterDevice;
    const matchesStatus = filterStatus === 'all' || session.status === filterStatus;

    let matchesDate = true;
    if (dateRange && dateRange[0] && dateRange[1]) {
      const sessionDate = dayjs(session.startTime);
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

    switch (device.type) {
      case DeviceType.SPEAKER: return <AudioOutlined />;
      case DeviceType.DISPLAY: return <DesktopOutlined />;
      case DeviceType.HUB: return <ControlOutlined />;
      default: return <AudioOutlined />;
    }
  };

  // 获取状态颜色
  const getStatusColor = (status: SessionStatus) => {
    switch (status) {
      case SessionStatus.ACTIVE: return 'processing';
      case SessionStatus.COMPLETED: return 'success';
      case SessionStatus.INTERRUPTED: return 'warning';
      default: return 'default';
    }
  };

  // 获取状态文本
  const getStatusText = (status: SessionStatus) => {
    switch (status) {
      case SessionStatus.ACTIVE: return '进行中';
      case SessionStatus.COMPLETED: return '已完成';
      case SessionStatus.INTERRUPTED: return '已中断';
      default: return '未知';
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
      title: '时间',
      dataIndex: 'startTime',
      key: 'startTime',
      render: (startTime: string) => (
        <div>
          <div>{dayjs(startTime).format('YYYY-MM-DD')}</div>
          <div style={{ fontSize: 12, color: '#8c8c8c' }}>
            {dayjs(startTime).format('HH:mm:ss')}
          </div>
        </div>
      ),
      sorter: (a, b) => new Date(a.startTime).getTime() - new Date(b.startTime).getTime(),
      defaultSortOrder: 'descend'
    },
    {
      title: '设备',
      dataIndex: 'deviceId',
      key: 'deviceId',
      render: (deviceId: string) => (
        <Space>
          <Avatar
            icon={getDeviceTypeIcon(deviceId)}
            size="small"
          />
          {getDeviceName(deviceId)}
        </Space>
      ),
      filters: devices.map(device => ({
        text: device.name,
        value: device.id
      })),
      onFilter: (value, record) => record.deviceId === value
    },
    {
      title: '用户输入',
      dataIndex: 'transcription',
      key: 'transcription',
      ellipsis: {
        showTitle: false
      },
      render: (transcription: string) => (
        <Tooltip placement="topLeft" title={transcription}>
          {transcription}
        </Tooltip>
      )
    },
    {
      title: '助手回复',
      dataIndex: 'response',
      key: 'response',
      ellipsis: {
        showTitle: false
      },
      render: (response: string) => (
        <Tooltip placement="topLeft" title={response}>
          {response}
        </Tooltip>
      )
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: SessionStatus) => (
        <Tag color={getStatusColor(status)}>
          {getStatusText(status)}
        </Tag>
      ),
      filters: [
        { text: '进行中', value: SessionStatus.ACTIVE },
        { text: '已完成', value: SessionStatus.COMPLETED },
        { text: '已中断', value: SessionStatus.INTERRUPTED }
      ],
      onFilter: (value, record) => record.status === value
    },
    {
      title: '时长',
      dataIndex: 'duration',
      key: 'duration',
      render: (duration?: number) => (
        duration ? (
          <Space>
            <ClockCircleOutlined />
            {duration}秒
          </Space>
        ) : '-'
      ),
      sorter: (a, b) => (a.duration || 0) - (b.duration || 0)
    },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Button
          type="link"
          icon={<PlayCircleOutlined />}
          onClick={() => handleViewSession(record)}
        >
          查看详情
        </Button>
      )
    }
  ];

  return (
    <div style={{ padding: 24 }}>
      {/* 统计卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="今日会话"
              value={stats.totalToday}
              prefix={<PlayCircleOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="活跃会话"
              value={stats.activeNow}
              valueStyle={{ color: '#1890ff' }}
              prefix={<PlayCircleOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="平均时长"
              value={stats.averageDuration}
              suffix="秒"
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* 搜索和过滤 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Input
            placeholder="搜索会话内容"
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 200 }}
          />
          <Select
            placeholder="设备筛选"
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
            placeholder="状态筛选"
            value={filterStatus}
            onChange={setFilterStatus}
            style={{ width: 120 }}
          >
            <Select.Option value="all">全部状态</Select.Option>
            <Select.Option value={SessionStatus.ACTIVE}>进行中</Select.Option>
            <Select.Option value={SessionStatus.COMPLETED}>已完成</Select.Option>
            <Select.Option value={SessionStatus.INTERRUPTED}>已中断</Select.Option>
          </Select>
          <RangePicker
            value={dateRange}
            onChange={setDateRange}
            placeholder={['开始日期', '结束日期']}
          />
          <Button icon={<ReloadOutlined />} onClick={() => fetchSessions()} loading={loading}>
            刷新
          </Button>
        </Space>
      </Card>

      {/* 会话列表 */}
      <Card>
        <Table
          columns={columns}
          dataSource={filteredSessions}
          rowKey="id"
          loading={loading}
          pagination={{
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total, range) =>
              `第 ${range[0]}-${range[1]} 条，共 ${total} 条记录`
          }}
        />
      </Card>

      {/* 会话详情对话框 */}
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
          <div>
            <Descriptions column={1} bordered>
              <Descriptions.Item label="会话ID">
                {selectedSession.id}
              </Descriptions.Item>
              <Descriptions.Item label="设备">
                <Space>
                  <Avatar
                    icon={getDeviceTypeIcon(selectedSession.deviceId)}
                    size="small"
                  />
                  {getDeviceName(selectedSession.deviceId)}
                </Space>
              </Descriptions.Item>
              <Descriptions.Item label="开始时间">
                {dayjs(selectedSession.startTime).format('YYYY-MM-DD HH:mm:ss')}
              </Descriptions.Item>
              {selectedSession.endTime && (
                <Descriptions.Item label="结束时间">
                  {dayjs(selectedSession.endTime).format('YYYY-MM-DD HH:mm:ss')}
                </Descriptions.Item>
              )}
              {selectedSession.duration && (
                <Descriptions.Item label="会话时长">
                  {selectedSession.duration} 秒
                </Descriptions.Item>
              )}
              <Descriptions.Item label="状态">
                <Tag color={getStatusColor(selectedSession.status)}>
                  {getStatusText(selectedSession.status)}
                </Tag>
              </Descriptions.Item>
            </Descriptions>

            <div style={{ marginTop: 24 }}>
              <h4>对话内容</h4>
              <Timeline>
                <Timeline.Item color="blue">
                  <div>
                    <div style={{ fontWeight: 500, marginBottom: 4 }}>
                      <UserOutlined /> 用户输入
                    </div>
                    <div style={{
                      padding: 12,
                      background: '#f0f2f5',
                      borderRadius: 6,
                      marginTop: 4
                    }}>
                      {selectedSession.transcription}
                    </div>
                  </div>
                </Timeline.Item>
                <Timeline.Item color="green">
                  <div>
                    <div style={{ fontWeight: 500, marginBottom: 4 }}>
                      <AudioOutlined /> 智能助手
                    </div>
                    <div style={{
                      padding: 12,
                      background: '#f6ffed',
                      borderRadius: 6,
                      border: '1px solid #b7eb8f',
                      marginTop: 4
                    }}>
                      {selectedSession.response}
                    </div>
                  </div>
                </Timeline.Item>
              </Timeline>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
};
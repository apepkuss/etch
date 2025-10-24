import React, { useEffect, useState } from 'react';
import {
  Card,
  Table,
  Button,
  Tag,
  Space,
  Input,
  Select,
  Modal,
  Form,
  message,
  Popconfirm,
  Avatar,
  Progress,
  Tooltip
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  ReloadOutlined,
  SettingOutlined,
  SearchOutlined,
  WifiOutlined,
  BatteryOutlined,
  AudioOutlined,
  DesktopOutlined,
  ControlOutlined
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { useDeviceStore } from '../stores/useDeviceStore';
import { Device, DeviceStatus, DeviceType, DeviceFormData } from '../types';
import type { ColumnsType } from 'antd/es/table';

export const DeviceList: React.FC = () => {
  const navigate = useNavigate();
  const {
    devices,
    loading,
    error,
    fetchDevices,
    deleteDevice,
    restartDevice,
    addDevice
  } = useDeviceStore();

  const [searchText, setSearchText] = useState('');
  const [filterStatus, setFilterStatus] = useState<DeviceStatus | 'all'>('all');
  const [filterType, setFilterType] = useState<DeviceType | 'all'>('all');
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [form] = Form.useForm();

  useEffect(() => {
    fetchDevices();
  }, [fetchDevices]);

  // 过滤设备
  const filteredDevices = devices.filter(device => {
    const matchesSearch = device.name.toLowerCase().includes(searchText.toLowerCase()) ||
                         device.location.toLowerCase().includes(searchText.toLowerCase());
    const matchesStatus = filterStatus === 'all' || device.status === filterStatus;
    const matchesType = filterType === 'all' || device.type === filterType;

    return matchesSearch && matchesStatus && matchesType;
  });

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

  // 处理添加设备
  const handleAddDevice = async (values: DeviceFormData) => {
    try {
      await addDevice({
        ...values,
        status: DeviceStatus.OFFLINE,
        firmwareVersion: '1.0.0',
        volume: 50,
        lastSeen: new Date().toISOString(),
        isOnline: false
      });

      message.success('设备添加成功');
      setIsModalVisible(false);
      form.resetFields();
      fetchDevices();
    } catch (error) {
      message.error('添加设备失败');
    }
  };

  // 处理删除设备
  const handleDeleteDevice = async (deviceId: string) => {
    try {
      await deleteDevice(deviceId);
      message.success('设备删除成功');
    } catch (error) {
      message.error('删除设备失败');
    }
  };

  // 处理重启设备
  const handleRestartDevice = async (deviceId: string) => {
    try {
      await restartDevice(deviceId);
      message.success('设备重启指令已发送');
    } catch (error) {
      message.error('重启设备失败');
    }
  };

  // 表格列定义
  const columns: ColumnsType<Device> = [
    {
      title: '设备信息',
      key: 'device',
      render: (_, record) => (
        <Space>
          <Avatar
            icon={getDeviceTypeIcon(record.type)}
            style={{
              backgroundColor: record.isOnline ? '#1890ff' : '#d9d9d9'
            }}
          />
          <div>
            <div style={{ fontWeight: 500 }}>{record.name}</div>
            <div style={{ fontSize: 12, color: '#8c8c8c' }}>
              {record.location} • {record.id}
            </div>
          </div>
        </Space>
      )
    },
    {
      title: '类型',
      dataIndex: 'type',
      key: 'type',
      render: (type: DeviceType) => {
        const typeMap = {
          [DeviceType.SPEAKER]: '智能音箱',
          [DeviceType.DISPLAY]: '智能显示屏',
          [DeviceType.HUB]: '中控设备'
        };
        return typeMap[type] || '未知设备';
      }
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: DeviceStatus) => (
        <Tag color={getStatusColor(status)}>
          {getStatusText(status)}
        </Tag>
      )
    },
    {
      title: '电量',
      dataIndex: 'batteryLevel',
      key: 'batteryLevel',
      render: (batteryLevel?: number) => (
        batteryLevel !== undefined ? (
          <Tooltip title={`${batteryLevel}%`}>
            <Progress
              percent={batteryLevel}
              size="small"
              format={() => (
                <span style={{ fontSize: 12 }}>
                  <BatteryOutlined /> {batteryLevel}%
                </span>
              )}
              strokeColor={batteryLevel > 20 ? '#52c41a' : '#ff4d4f'}
            />
          </Tooltip>
        ) : '-'
      )
    },
    {
      title: '音量',
      dataIndex: 'volume',
      key: 'volume',
      render: (volume: number) => `${volume}%`
    },
    {
      title: '固件版本',
      dataIndex: 'firmwareVersion',
      key: 'firmwareVersion'
    },
    {
      title: '最后在线',
      dataIndex: 'lastSeen',
      key: 'lastSeen',
      render: (lastSeen: string) => (
        <Tooltip title={new Date(lastSeen).toLocaleString()}>
          {new Date(lastSeen).toLocaleTimeString()}
        </Tooltip>
      )
    },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Tooltip title="查看详情">
            <Button
              type="text"
              icon={<SettingOutlined />}
              onClick={() => navigate(`/devices/${record.id}`)}
            />
          </Tooltip>
          <Tooltip title="重启设备">
            <Popconfirm
              title="确定要重启此设备吗？"
              onConfirm={() => handleRestartDevice(record.id)}
              disabled={record.status === DeviceStatus.MAINTENANCE}
            >
              <Button
                type="text"
                icon={<ReloadOutlined />}
                disabled={record.status === DeviceStatus.MAINTENANCE}
              />
            </Popconfirm>
          </Tooltip>
          <Tooltip title="删除设备">
            <Popconfirm
              title="确定要删除此设备吗？此操作不可恢复。"
              onConfirm={() => handleDeleteDevice(record.id)}
            >
              <Button
                type="text"
                danger
                icon={<DeleteOutlined />}
              />
            </Popconfirm>
          </Tooltip>
        </Space>
      )
    }
  ];

  return (
    <div style={{ padding: 24 }}>
      {/* 页面头部 */}
      <Card style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h2 style={{ margin: 0, marginBottom: 8 }}>设备管理</h2>
            <p style={{ margin: 0, color: '#8c8c8c' }}>
              共 {devices.length} 个设备，{filteredDevices.length} 个显示
            </p>
          </div>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => setIsModalVisible(true)}
          >
            添加设备
          </Button>
        </div>
      </Card>

      {/* 搜索和过滤 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Input
            placeholder="搜索设备名称或位置"
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 200 }}
          />
          <Select
            placeholder="状态筛选"
            value={filterStatus}
            onChange={setFilterStatus}
            style={{ width: 120 }}
          >
            <Select.Option value="all">全部状态</Select.Option>
            <Select.Option value={DeviceStatus.ONLINE}>在线</Select.Option>
            <Select.Option value={DeviceStatus.OFFLINE}>离线</Select.Option>
            <Select.Option value={DeviceStatus.ERROR}>故障</Select.Option>
            <Select.Option value={DeviceStatus.MAINTENANCE}>维护中</Select.Option>
          </Select>
          <Select
            placeholder="类型筛选"
            value={filterType}
            onChange={setFilterType}
            style={{ width: 120 }}
          >
            <Select.Option value="all">全部类型</Select.Option>
            <Select.Option value={DeviceType.SPEAKER}>智能音箱</Select.Option>
            <Select.Option value={DeviceType.DISPLAY}>智能显示屏</Select.Option>
            <Select.Option value={DeviceType.HUB}>中控设备</Select.Option>
          </Select>
          <Button icon={<ReloadOutlined />} onClick={fetchDevices} loading={loading}>
            刷新
          </Button>
        </Space>
      </Card>

      {/* 设备列表 */}
      <Card>
        <Table
          columns={columns}
          dataSource={filteredDevices}
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

      {/* 添加设备对话框 */}
      <Modal
        title="添加设备"
        open={isModalVisible}
        onCancel={() => {
          setIsModalVisible(false);
          form.resetFields();
        }}
        onOk={() => form.submit()}
        okText="添加"
        cancelText="取消"
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleAddDevice}
        >
          <Form.Item
            name="name"
            label="设备名称"
            rules={[{ required: true, message: '请输入设备名称' }]}
          >
            <Input placeholder="请输入设备名称" />
          </Form.Item>

          <Form.Item
            name="type"
            label="设备类型"
            rules={[{ required: true, message: '请选择设备类型' }]}
          >
            <Select placeholder="请选择设备类型">
              <Select.Option value={DeviceType.SPEAKER}>智能音箱</Select.Option>
              <Select.Option value={DeviceType.DISPLAY}>智能显示屏</Select.Option>
              <Select.Option value={DeviceType.HUB}>中控设备</Select.Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="location"
            label="设备位置"
            rules={[{ required: true, message: '请输入设备位置' }]}
          >
            <Input placeholder="请输入设备位置" />
          </Form.Item>

          <Form.Item
            name="owner"
            label="设备所有者"
            rules={[{ required: true, message: '请输入设备所有者' }]}
          >
            <Input placeholder="请输入设备所有者" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};
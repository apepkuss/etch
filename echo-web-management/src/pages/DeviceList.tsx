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

  // 显示错误消息
  useEffect(() => {
    if (error) {
      message.error(error);
    }
  }, [error]);

  // 过滤设备
  const filteredDevices = devices.filter(device => {
    const matchesSearch = device.name.toLowerCase().includes(searchText.toLowerCase()) ||
                         device.location.toLowerCase().includes(searchText.toLowerCase());
    const matchesStatus = filterStatus === 'all' || device.status === filterStatus;
    const matchesType = filterType === 'all' || device.device_type === filterType;

    return matchesSearch && matchesStatus && matchesType;
  });

  // 获取状态颜色
  const getStatusColor = (status: DeviceStatus) => {
    switch (status) {
      case DeviceStatus.Online: return 'success';
      case DeviceStatus.Offline: return 'default';
      case DeviceStatus.Error: return 'error';
      case DeviceStatus.Maintenance: return 'warning';
      default: return 'default';
    }
  };

  // 获取状态文本
  const getStatusText = (status: DeviceStatus) => {
    switch (status) {
      case DeviceStatus.Online: return '在线';
      case DeviceStatus.Offline: return '离线';
      case DeviceStatus.Error: return '故障';
      case DeviceStatus.Maintenance: return '维护中';
      default: return '未知';
    }
  };

  // 获取设备类型图标
  const getDeviceTypeIcon = (type: DeviceType) => {
    switch (type) {
      case DeviceType.Speaker: return <AudioOutlined />;
      case DeviceType.Display: return <DesktopOutlined />;
      case DeviceType.Hub: return <ControlOutlined />;
      default: return <AudioOutlined />;
    }
  };

  // 获取电池颜色
  const getBatteryColor = (level: number) => {
    if (level > 60) return '#52c41a';
    if (level > 30) return '#faad14';
    return '#ff4d4f';
  };

  // 刷新设备列表
  const handleRefresh = () => {
    fetchDevices();
    message.success('设备列表已刷新');
  };

  // 添加设备
  const handleAddDevice = async (values: DeviceFormData) => {
    try {
      await addDevice({
        ...values,
        device_type: values.type,
        status: DeviceStatus.Offline,
        firmware_version: '1.0.0',
        battery_level: 0,
        volume: 50,
        last_seen: new Date().toISOString(),
        is_online: false
      });

      setIsModalVisible(false);
      form.resetFields();
      message.success('设备添加成功');
    } catch (error) {
      message.error('添加设备失败');
    }
  };

  // 删除设备
  const handleDeleteDevice = async (deviceId: string) => {
    try {
      await deleteDevice(deviceId);
      message.success('设备删除成功');
    } catch (error) {
      message.error('删除设备失败');
    }
  };

  // 重启设备
  const handleRestartDevice = async (deviceId: string) => {
    try {
      await restartDevice(deviceId);
      message.success('设备重启命令已发送');
    } catch (error) {
      message.error('重启设备失败');
    }
  };

  // 表格列定义
  const columns: ColumnsType<Device> = [
    {
      title: '设备',
      key: 'device',
      render: (_, record) => (
        <Space>
          <Avatar
            icon={getDeviceTypeIcon(record.device_type)}
            style={{ backgroundColor: record.is_online ? '#52c41a' : '#d9d9d9' }}
          />
          <div>
            <div style={{ fontWeight: 'bold' }}>{record.name}</div>
            <div style={{ color: '#666', fontSize: '12px' }}>{record.id}</div>
          </div>
        </Space>
      )
    },
    {
      title: '类型',
      dataIndex: 'device_type',
      key: 'device_type',
      render: (type: DeviceType) => (
        <Space>
          {getDeviceTypeIcon(type)}
          <span>{type}</span>
        </Space>
      )
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: DeviceStatus, record) => (
        <Space direction="vertical" size="small">
          <Tag color={getStatusColor(status)}>
            {getStatusText(status)}
          </Tag>
          {record.is_online && (
            <Tag color="green" icon={<WifiOutlined />}>
              连接中
            </Tag>
          )}
        </Space>
      )
    },
    {
      title: '位置',
      dataIndex: 'location',
      key: 'location'
    },
    {
      title: '电池',
      dataIndex: 'battery_level',
      key: 'battery_level',
      render: (level: number) => (
        <Progress
          percent={level}
          size="small"
          strokeColor={getBatteryColor(level)}
          format={() => `${level}%`}
        />
      )
    },
    {
      title: '音量',
      dataIndex: 'volume',
      key: 'volume',
      render: (volume: number) => (
        <Progress
          percent={volume}
          size="small"
          format={() => `${volume}%`}
        />
      )
    },
    {
      title: '固件版本',
      dataIndex: 'firmware_version',
      key: 'firmware_version'
    },
    {
      title: '最后在线',
      dataIndex: 'last_seen',
      key: 'last_seen',
      render: (time: string) => (
        <Tooltip title={new Date(time).toLocaleString()}>
          {new Date(time).toLocaleDateString()}
        </Tooltip>
      )
    },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Tooltip title="设备详情">
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
              okText="确定"
              cancelText="取消"
            >
              <Button type="text" icon={<ReloadOutlined />} />
            </Popconfirm>
          </Tooltip>
          <Tooltip title="删除设备">
            <Popconfirm
              title="确定要删除此设备吗？"
              onConfirm={() => handleDeleteDevice(record.id)}
              okText="确定"
              cancelText="取消"
            >
              <Button type="text" danger icon={<DeleteOutlined />} />
            </Popconfirm>
          </Tooltip>
        </Space>
      )
    }
  ];

  return (
    <div style={{ padding: '24px' }}>
      <Card
        title="设备管理"
        extra={
          <Space>
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setIsModalVisible(true)}
            >
              添加设备
            </Button>
            <Button
              icon={<ReloadOutlined />}
              onClick={handleRefresh}
              loading={loading}
            >
              刷新
            </Button>
          </Space>
        }
      >
        {/* 搜索和过滤 */}
        <Space style={{ marginBottom: '16px' }}>
          <Input
            placeholder="搜索设备名称或位置"
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 200 }}
          />
          <Select
            placeholder="状态过滤"
            value={filterStatus}
            onChange={setFilterStatus}
            style={{ width: 120 }}
          >
            <Select.Option value="all">全部状态</Select.Option>
            <Select.Option value={DeviceStatus.Online}>在线</Select.Option>
            <Select.Option value={DeviceStatus.Offline}>离线</Select.Option>
            <Select.Option value={DeviceStatus.Error}>故障</Select.Option>
            <Select.Option value={DeviceStatus.Maintenance}>维护中</Select.Option>
          </Select>
          <Select
            placeholder="类型过滤"
            value={filterType}
            onChange={setFilterType}
            style={{ width: 120 }}
          >
            <Select.Option value="all">全部类型</Select.Option>
            <Select.Option value={DeviceType.Speaker}>音箱</Select.Option>
            <Select.Option value={DeviceType.Display}>显示屏</Select.Option>
            <Select.Option value={DeviceType.Hub}>中控</Select.Option>
          </Select>
        </Space>

        {/* 设备表格 */}
        <Table
          columns={columns}
          dataSource={filteredDevices}
          rowKey="id"
          loading={loading}
          pagination={{
            total: filteredDevices.length,
            pageSize: 10,
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total, range) => `第 ${range[0]}-${range[1]} 条，共 ${total} 条`
          }}
        />
      </Card>

      {/* 添加设备模态框 */}
      <Modal
        title="添加设备"
        open={isModalVisible}
        onOk={() => form.submit()}
        onCancel={() => {
          setIsModalVisible(false);
          form.resetFields();
        }}
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
              <Select.Option value={DeviceType.Speaker}>音箱</Select.Option>
              <Select.Option value={DeviceType.Display}>显示屏</Select.Option>
              <Select.Option value={DeviceType.Hub}>中控</Select.Option>
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

export default DeviceList;
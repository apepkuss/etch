import React, { useState, useEffect } from 'react';
import {
  Card,
  Button,
  Table,
  Tag,
  Space,
  Input,
  Select,
  Form,
  message,
  Popconfirm,
  Avatar,
  Progress,
  Tooltip,
  Spin
} from 'antd';
import {
  DeleteOutlined,
  ReloadOutlined,
  SettingOutlined,
  SearchOutlined,
  WifiOutlined,
  AudioOutlined,
  DesktopOutlined,
  ControlOutlined,
  QrcodeOutlined
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import DeviceRegistrationModal from '../components/DeviceRegistrationModal';
import useDeviceStore from '../stores/useDeviceStore';
import type { ColumnsType } from 'antd/es/table';
import { Device, DeviceStatus, DeviceType } from '../types';

const DeviceList: React.FC = () => {
  const navigate = useNavigate();
  const [searchText, setSearchText] = useState('');
  const [filterStatus, setFilterStatus] = useState<DeviceStatus | 'all'>('all');
  const [filterType, setFilterType] = useState<DeviceType | 'all'>('all');
  const [isRegistrationModalVisible, setIsRegistrationModalVisible] = useState(false);
  const [form] = Form.useForm();

  // 使用设备存储
  const {
    devices,
    loading,
    error,
    fetchDevices,
    deleteDevice: deleteDeviceFromStore,
    fetchDeviceStats
  } = useDeviceStore();

  // 组件挂载时获取设备数据
  useEffect(() => {
    fetchDevices();
    fetchDeviceStats();
  }, [fetchDevices, fetchDeviceStats]);

  // 错误处理
  useEffect(() => {
    if (error) {
      message.error(`获取设备数据失败: ${error}`);
    }
  }, [error]);

  // 刷新处理
  const handleRefresh = async () => {
    try {
      await fetchDevices();
      await fetchDeviceStats();
      message.success('设备列表已刷新');
    } catch (error) {
      message.error('刷新失败');
      console.error('Failed to refresh devices:', error);
    }
  };

  // 处理注册成功
  const handleRegistrationSuccess = async () => {
    try {
      await fetchDevices();
      await fetchDeviceStats();
      message.success('设备注册成功，设备已添加到列表中');
    } catch (error) {
      console.error('Failed to refresh devices after registration:', error);
    }
  };

  // 获取状态标签
  const getStatusTag = (status: DeviceStatus) => {
    const statusConfig = {
      [DeviceStatus.Online]: { color: 'success', text: '在线' },
      [DeviceStatus.Offline]: { color: 'default', text: '离线' },
      [DeviceStatus.Maintenance]: { color: 'warning', text: '维护中' },
      [DeviceStatus.Error]: { color: 'error', text: '错误' },
      [DeviceStatus.Pending]: { color: 'processing', text: '待注册' },
      [DeviceStatus.RegistrationExpired]: { color: 'error', text: '注册过期' }
    };

    const config = statusConfig[status] || { color: 'default', text: '未知' };
    return <Tag color={config.color}>{config.text}</Tag>;
  };

  // 获取设备类型图标
  const getDeviceIcon = (type: DeviceType) => {
    const iconMap = {
      [DeviceType.Speaker]: <AudioOutlined />,
      [DeviceType.Display]: <DesktopOutlined />,
      [DeviceType.Hub]: <ControlOutlined />
    };
    return iconMap[type] || <AudioOutlined />;
  };

  // 表格列定义
  const columns: ColumnsType<Device> = [
    {
      title: '设备名称',
      dataIndex: 'name',
      key: 'name',
      align: 'center',
      render: (text: string, record: Device) => (
        <Space>
          <Avatar icon={getDeviceIcon(record.device_type)} />
          <div>
            <div style={{ fontWeight: 500 }}>{text}</div>
            <div style={{ fontSize: 12, color: '#8c8c8c' }}>{record.id}</div>
          </div>
        </Space>
      ),
    },
    {
      title: '类型',
      dataIndex: 'device_type',
      key: 'device_type',
      align: 'center',
      render: (type: DeviceType) => {
        const typeMap = {
          [DeviceType.Speaker]: '智能音箱'
        };
        return typeMap[type] || '未知设备';
      },
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      align: 'center',
      render: (status: DeviceStatus) => getStatusTag(status),
    },
    {
      title: '电量',
      dataIndex: 'battery_level',
      key: 'battery_level',
      align: 'center',
      render: (level: number) => (
        <Progress
          percent={level}
          size="small"
          status={level < 20 ? 'exception' : level < 50 ? 'active' : 'success'}
          format={percent => `${percent}%`}
        />
      ),
    },
    {
      title: '操作',
      key: 'actions',
      align: 'center',
      render: (_, record: Device) => (
        <Space>
          <Button
            type="primary"
            size="small"
            icon={<SettingOutlined />}
            onClick={() => navigate(`/devices/${record.id}`)}
          >
            配置
          </Button>
          <Popconfirm
            title="确定要删除这个设备吗？"
            onConfirm={async () => {
              try {
                await deleteDeviceFromStore(record.id);
                message.success('设备已删除');
              } catch (error) {
                message.error('删除设备失败');
                console.error('Failed to delete device:', error);
              }
            }}
            okText="确定"
            cancelText="取消"
          >
            <Button
              danger
              size="small"
              icon={<DeleteOutlined />}
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  // 过滤设备数据
  const filteredDevices = (devices || []).filter(device => {
    const matchesSearch = device.name.toLowerCase().includes(searchText.toLowerCase()) ||
                         device.id.toLowerCase().includes(searchText.toLowerCase());
    const matchesStatus = filterStatus === 'all' || device.status === filterStatus;
    const matchesType = filterType === 'all' || device.device_type === filterType;

    return matchesSearch && matchesStatus && matchesType;
  });

  return (
    <div style={{ padding: 24 }}>
      <Card
        title="设备管理"
        extra={
          <Space>
            <Button
              type="primary"
              icon={<QrcodeOutlined />}
              onClick={() => setIsRegistrationModalVisible(true)}
            >
              注册新设备
            </Button>
            <Button
              icon={<ReloadOutlined />}
              onClick={handleRefresh}
            >
              刷新
            </Button>
          </Space>
        }
      >
        {/* 搜索和过滤 */}
        <div style={{ marginBottom: 16 }}>
          <Space>
            <Input
              placeholder="搜索设备名称或ID"
              prefix={<SearchOutlined />}
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              style={{ width: 250 }}
            />
            <Select
              placeholder="设备状态"
              value={filterStatus}
              onChange={setFilterStatus}
              style={{ width: 120 }}
            >
              <Select.Option value="all">全部状态</Select.Option>
              <Select.Option value={DeviceStatus.Online}>在线</Select.Option>
              <Select.Option value={DeviceStatus.Offline}>离线</Select.Option>
              <Select.Option value={DeviceStatus.Maintenance}>维护中</Select.Option>
              <Select.Option value={DeviceStatus.Error}>错误</Select.Option>
              <Select.Option value={DeviceStatus.Pending}>待注册</Select.Option>
            </Select>
            <Select
              placeholder="设备类型"
              value={filterType}
              onChange={setFilterType}
              style={{ width: 120 }}
            >
              <Select.Option value="all">全部类型</Select.Option>
              <Select.Option value={DeviceType.Speaker}>智能音箱</Select.Option>
            </Select>
          </Space>
        </div>

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
            showTotal: (total) => `共 ${total} 个设备`,
          }}
        />
      </Card>

      {/* 设备注册模态框 */}
      <DeviceRegistrationModal
        visible={isRegistrationModalVisible}
        onClose={() => setIsRegistrationModalVisible(false)}
        onSuccess={handleRegistrationSuccess}
      />
    </div>
  );
};

export default DeviceList;
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
import useEchoKitServerStore from '../stores/useEchoKitServerStore';
import type { ColumnsType } from 'antd/es/table';
import { Device, DeviceStatus, DeviceType } from '../types';

const DeviceList: React.FC = () => {
  const navigate = useNavigate();
  const [searchText, setSearchText] = useState('');
  const [filterStatus, setFilterStatus] = useState<DeviceStatus | 'all'>('all');
  const [filterType, setFilterType] = useState<DeviceType | 'all'>('all');
  const [isRegistrationModalVisible, setIsRegistrationModalVisible] = useState(false);
  const [form] = Form.useForm();

  // 跟踪正在编辑 EchoKit Server 的设备
  const [editingDeviceId, setEditingDeviceId] = useState<string | null>(null);
  const [tempServerUrl, setTempServerUrl] = useState<string | undefined>(undefined);

  // 使用设备存储
  const {
    devices,
    loading,
    error,
    fetchDevices,
    deleteDevice: deleteDeviceFromStore,
    updateDevice,
    fetchDeviceStats
  } = useDeviceStore();

  // 使用 EchoKit Server 存储
  const {
    servers: echokitServers,
    fetchServers: fetchEchokitServers
  } = useEchoKitServerStore();

  // 组件挂载时获取设备数据
  useEffect(() => {
    fetchDevices();
    fetchDeviceStats();
    fetchEchokitServers();
  }, [fetchDevices, fetchDeviceStats, fetchEchokitServers]);

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

  // 保存 EchoKit Server URL 更新
  const handleSaveServerUrl = async (deviceId: string) => {
    try {
      const beforeDevice = devices.find(d => d.id === deviceId);
      console.log('[DeviceList] Before save:', {
        deviceId,
        tempServerUrl,
        'beforeDevice.echokit_server_url': beforeDevice?.echokit_server_url
      });

      await updateDevice(deviceId, { echokit_server_url: tempServerUrl });

      // 注意：这里读取的 devices 可能还是旧的，因为 React 状态更新是异步的
      const afterDevice = devices.find(d => d.id === deviceId);
      console.log('[DeviceList] After save (may be stale):', {
        'afterDevice.echokit_server_url': afterDevice?.echokit_server_url
      });

      message.success('EchoKit 服务器已更新');
      setEditingDeviceId(null);
      setTempServerUrl(undefined);
      // updateDevice 已经更新了本地状态，不需要重新 fetchDevices
    } catch (error) {
      message.error('更新 EchoKit 服务器失败');
      console.error('Failed to update echokit server:', error);
    }
  };

  // 取消 EchoKit Server URL 更新
  const handleCancelServerUrl = () => {
    setEditingDeviceId(null);
    setTempServerUrl(undefined);
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
      title: 'EchoKit 服务器',
      dataIndex: 'echokit_server_url',
      key: 'echokit_server_url',
      align: 'center',
      render: (url: string | undefined, record: Device) => {
        const isEditing = editingDeviceId === record.id;
        const displayValue = isEditing ? tempServerUrl : url;

        return (
          <Space>
            <Select
              style={{ width: 200 }}
              placeholder="选择 EchoKit 服务器"
              value={displayValue}
              onChange={(value) => {
                setEditingDeviceId(record.id);
                setTempServerUrl(value);
              }}
              allowClear
            >
              {echokitServers.map((server) => (
                <Select.Option key={server.id} value={server.server_url}>
                  {server.server_url}
                </Select.Option>
              ))}
            </Select>
            {isEditing && (
              <>
                <Button
                  size="small"
                  type="primary"
                  onClick={() => handleSaveServerUrl(record.id)}
                >
                  保存
                </Button>
                <Button
                  size="small"
                  onClick={handleCancelServerUrl}
                >
                  取消
                </Button>
              </>
            )}
          </Space>
        );
      },
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
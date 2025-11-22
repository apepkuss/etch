import React, { useState, useEffect, useCallback } from 'react';
import {
  Modal,
  Form,
  Input,
  Select,
  Button,
  Steps,
  Card,
  Typography,
  Space,
  Alert,
  Spin,
  message,
  notification,
  Descriptions,
  Tag,
  QRCode,
  Tooltip,
  Tabs,
} from 'antd';
import {
  QrcodeOutlined,
  KeyOutlined,
  ReloadOutlined,
  CopyOutlined,
  CheckCircleOutlined,
  RollbackOutlined,
} from '@ant-design/icons';
import { devicesApi } from '../api/devices';
import { echokitServersApi, EchoKitServer } from '../api/echokitServers';
import {
  DeviceRegistrationRequest,
  DeviceRegistrationResponse,
  DeviceType,
} from '../types';

const { Title, Text, Paragraph } = Typography;
const { Option } = Select;

interface DeviceRegistrationModalProps {
  visible: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

const DeviceRegistrationModal: React.FC<DeviceRegistrationModalProps> = ({
  visible,
  onClose,
  onSuccess,
}) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [registrationData, setRegistrationData] = useState<DeviceRegistrationResponse | null>(null);
  const [timeLeft, setTimeLeft] = useState(15 * 60); // 15分钟
  const [registrationStatus, setRegistrationStatus] = useState<'active' | 'success' | 'failed' | 'expired'>('active');
  const [createdDeviceInfo, setCreatedDeviceInfo] = useState<any>(null); // 新增：存储创建的设备信息
  const [registrationMethod, setRegistrationMethod] = useState<'scan' | 'manual'>('scan'); // 新增：注册方式选择
  const [echokitServers, setEchokitServers] = useState<EchoKitServer[]>([]); // EchoKit 服务器列表
  const [loadingServers, setLoadingServers] = useState(false); // 加载服务器列表状态
  const [form] = Form.useForm();

  // 重置所有状态的函数
  const resetModalState = useCallback(() => {
    setCurrentStep(0);
    setLoading(false);
    setRegistrationData(null);
    setTimeLeft(15 * 60);
    setRegistrationStatus('active');
    setCreatedDeviceInfo(null);
    setRegistrationMethod('scan');
    form.resetFields();
  }, [form]);

  // 加载 EchoKit 服务器列表
  const loadEchokitServers = useCallback(async () => {
    setLoadingServers(true);
    try {
      const servers = await echokitServersApi.getServers();
      setEchokitServers(servers);
    } catch (error) {
      console.error('Failed to load EchoKit servers:', error);
      message.error('加载 EchoKit 服务器列表失败');
    } finally {
      setLoadingServers(false);
    }
  }, []);

  // 当 modal 打开时加载服务器列表，关闭时重置状态
  useEffect(() => {
    if (visible) {
      loadEchokitServers();
    } else {
      resetModalState();
    }
  }, [visible, resetModalState, loadEchokitServers]);

  // 当服务器列表加载完成后，设置默认选中第一个服务器
  useEffect(() => {
    if (echokitServers.length > 0 && !form.getFieldValue('echokit_server_url')) {
      form.setFieldValue('echokit_server_url', echokitServers[0].server_url);
    }
  }, [echokitServers, form]);

  // 设备类型选项
  const deviceTypeOptions = [
    { label: '🔊 智能音箱', value: DeviceType.Speaker }
  ];

  // 计算剩余时间
  const calculateTimeLeft = useCallback((expiresAt: string) => {
    const now = new Date().getTime();
    const expires = new Date(expiresAt).getTime();
    return Math.max(0, Math.floor((expires - now) / 1000));
  }, []);

  // 倒计时处理
  useEffect(() => {
    if (registrationData && registrationStatus === 'active') {
      const timeUntilExpiry = calculateTimeLeft(registrationData.expires_at);
      setTimeLeft(timeUntilExpiry);

      const timer = setInterval(() => {
        const newTimeLeft = calculateTimeLeft(registrationData.expires_at);
        setTimeLeft(newTimeLeft);

        if (newTimeLeft <= 0) {
          handleTokenExpired();
          clearInterval(timer);
        } else if (newTimeLeft <= 60) {
          message.warning('配对码即将在1分钟后过期，请尽快完成注册');
        }
      }, 1000);

      return () => clearInterval(timer);
    }
  }, [registrationData, registrationStatus, calculateTimeLeft]);

  // 格式化时间显示
  const formatTime = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  // 复制到剪贴板
  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      message.success('配对码已复制到剪贴板');
    } catch (error) {
      message.error('复制失败，请手动复制');
    }
  };

  // 数据库确认处理
  const handleDatabaseConfirmation = () => {
    setCurrentStep(3); // 跳到具体的注册流程
    message.info('数据库信息已确认，开始设备注册流程');
  };

  // Tab切换处理
  const handleTabChange = (key: string) => {
    setRegistrationMethod(key as 'scan' | 'manual');
  };

  // 重新生成配对码
  const regenerateRegistration = async () => {
    if (!registrationData) return;

    setLoading(true);
    try {
      const serialNumber = form.getFieldValue('serial_number');
      const macAddress = form.getFieldValue('mac_address');

      // 生成新的device_id
      const deviceId = generateDeviceId(serialNumber, macAddress);

      const request: DeviceRegistrationRequest = {
        name: form.getFieldValue('name'),
        device_type: form.getFieldValue('device_type'),
        device_id: deviceId,
        serial_number: serialNumber,
        mac_address: macAddress,
      };

      const response = await devicesApi.registerDevice(request);
      setRegistrationData(response);
      setTimeLeft(15 * 60);
      setRegistrationStatus('active');
      message.success('配对码已重新生成');
    } catch (error) {
      message.error('重新生成失败');
    } finally {
      setLoading(false);
    }
  };

  // 延长注册时间
  const extendRegistration = async () => {
    if (!registrationData) return;

    try {
      const response = await devicesApi.extendRegistration(registrationData.device_id, {
        device_id: registrationData.device_id,
        extension_duration_minutes: 15,
      });

      if (response.success) {
        setRegistrationData({
          ...registrationData,
          expires_at: response.new_expires_at,
        });
        message.success(`注册时间已延长${response.extension_duration_minutes}分钟`);
      }
    } catch (error) {
      message.error('延长失败');
    }
  };

  // 处理令牌过期
  const handleTokenExpired = () => {
    setRegistrationStatus('expired');
    Modal.confirm({
      title: '🕐 注册已过期',
      content: (
        <div>
          <p>设备注册因超时已自动取消，原因可能是：</p>
          <ul>
            <li>设备未及时扫码或输入配对码</li>
            <li>网络连接问题</li>
            <li>设备操作异常</li>
          </ul>
          <p>您可以选择：</p>
        </div>
      ),
      width: 500,
      okText: '重新注册',
      cancelText: '稍后再试',
      onOk: () => {
        setCurrentStep(0);
        setRegistrationData(null);
        setRegistrationStatus('active');
      },
      onCancel: onClose,
    });
  };

  // 取消注册
  const cancelRegistration = async () => {
    if (!registrationData) return;

    try {
      await devicesApi.cancelRegistration(registrationData.device_id);
      message.info('注册已取消');
      onClose();
    } catch (error) {
      message.error('取消失败');
    }
  };

  // 生成设备ID
  const generateDeviceId = (serialNumber: string, macAddress: string): string => {
    return `ECHO_${serialNumber}_${macAddress}`;
  };

  // 验证MAC地址格式（小写无冒号）
  const validateMacAddress = (mac: string): boolean => {
    const macPattern = /^[0-9a-f]{12}$/;
    return macPattern.test(mac);
  };

  // 表单提交处理
  const handleFormSubmit = async (values: any) => {
    // 验证MAC地址格式
    if (!validateMacAddress(values.mac_address)) {
      message.error('MAC地址格式不正确，请使用小写无冒号格式：a1b2c3d4e5f6');
      return;
    }

    setLoading(true);
    try {
      // 生成device_id
      const deviceId = generateDeviceId(values.serial_number, values.mac_address);
      console.log('开始注册设备，设备ID:', deviceId);

      const request: DeviceRegistrationRequest = {
        name: values.name,
        device_type: values.device_type,
        device_id: deviceId, // 添加device_id
        serial_number: values.serial_number, // 添加SN
        mac_address: values.mac_address, // 添加MAC
      };

      console.log('发送注册请求:', request);
      const response = await devicesApi.registerDevice(request);
      console.log('注册响应:', response);

      setRegistrationData(response);

      // 存储创建的设备信息
      const deviceInfo = {
        device_id: deviceId,
        name: values.name,
        serial_number: values.serial_number,
        mac_address: values.mac_address,
        device_type: values.device_type,
      };
      setCreatedDeviceInfo(deviceInfo);

      // 直接跳转到注册成功
      setCurrentStep(4);
      message.success('设备注册成功！');
    } catch (error: any) {
      console.error('设备注册失败:', error);

      // 根据不同的错误类型显示不同的提示
      if (error.response) {
        const status = error.response.status;

        let errorMessage = '';
        switch (status) {
          case 409:
            errorMessage = '设备注册失败：序列号或MAC地址已存在，请使用不同的序列号或MAC地址';
            break;
          case 400:
            errorMessage = '设备注册失败：请求数据格式错误，请检查输入信息';
            break;
          case 401:
            errorMessage = '设备注册失败：未授权，请重新登录';
            break;
          case 403:
            errorMessage = '设备注册失败：权限不足';
            break;
          case 500:
            errorMessage = '设备注册失败：服务器内部错误，请稍后重试';
            break;
          default:
            errorMessage = `设备注册失败：服务器错误 (${status})，请稍后重试`;
        }

        // 使用原生 alert 确保错误消息显示
        alert(errorMessage);

        // 使用 notification 显示错误
        notification.error({
          message: '设备注册失败',
          description: errorMessage,
          duration: 5,
        });

        // 同时也尝试 message.error（作为备用）
        message.error(errorMessage);

      } else if (error.request) {
        const errorMessage = '设备注册失败：网络连接错误，请检查网络连接';
        alert(errorMessage);
        notification.error({
          message: '设备注册失败',
          description: errorMessage,
          duration: 5,
        });
        message.error(errorMessage);
      } else {
        const errorMessage = '设备注册失败，请重试';
        alert(errorMessage);
        notification.error({
          message: '设备注册失败',
          description: errorMessage,
          duration: 5,
        });
        message.error(errorMessage);
      }
    } finally {
      setLoading(false);
    }
  };

  // 渲染设备信息表单
  const renderDeviceInfoForm = () => (
    <Form
      form={form}
      layout="vertical"
      onFinish={handleFormSubmit}
      initialValues={{
        device_type: DeviceType.Speaker,
        echokit_server_url: echokitServers.length > 0 ? echokitServers[0].server_url : undefined,
      }}
    >
      {/* 必填项 */}
      <Form.Item
        name="name"
        label="设备名称"
        rules={[
          { required: true, message: '请输入设备名称' },
          { min: 1, max: 100, message: '设备名称长度为1-100个字符' },
        ]}
      >
        <Input placeholder="给设备起个名字，如：客厅音箱" />
      </Form.Item>

      <Form.Item
        name="serial_number"
        label="设备序列号 (SN)"
        rules={[
          { required: true, message: '请输入设备序列号' },
          { min: 3, max: 50, message: '序列号长度为3-50个字符' },
          { pattern: /^[A-Za-z0-9_-]+$/, message: '序列号只能包含字母、数字、下划线和横线' },
        ]}
      >
        <Input
          placeholder="例如：ES20240115001"
          addonBefore="SN"
          onChange={(e) => {
            const serialNumber = e.target.value;
            const macAddress = form.getFieldValue('mac_address');
            if (serialNumber && macAddress) {
              const deviceId = generateDeviceId(serialNumber, macAddress);
              form.setFieldValue('device_id_preview', deviceId);
            }
          }}
        />
      </Form.Item>

      <Form.Item
        name="mac_address"
        label="MAC地址"
        rules={[
          { required: true, message: '请输入MAC地址' },
          {
            validator: (_, value) => {
              if (!value) return Promise.resolve();
              if (validateMacAddress(value)) {
                return Promise.resolve();
              }
              return Promise.reject(new Error('MAC地址格式不正确，请使用小写无冒号格式：a1b2c3d4e5f6'));
            },
          },
        ]}
      >
        <Input
          placeholder="例如：a1b2c3d4e5f6"
          addonBefore="MAC"
          style={{ textTransform: 'lowercase' }}
          onChange={(e) => {
            const macAddress = e.target.value.toLowerCase();
            e.target.value = macAddress;
            form.setFieldValue('mac_address', macAddress);
            const serialNumber = form.getFieldValue('serial_number');
            if (serialNumber && macAddress) {
              const deviceId = generateDeviceId(serialNumber, macAddress);
              form.setFieldValue('device_id_preview', deviceId);
            }
          }}
        />
      </Form.Item>

  
      {/* 设备类型 */}
      <Form.Item
        name="device_type"
        label="设备类型"
        rules={[
          { required: true, message: '请选择设备类型' },
        ]}
      >
        <Select placeholder="选择设备类型">
          {deviceTypeOptions.map(option => (
            <Option key={option.value} value={option.value}>
              {option.label}
            </Option>
          ))}
        </Select>
      </Form.Item>

      <Form.Item
        name="echokit_server_url"
        label="EchoKit 服务器"
        rules={[
          { required: true, message: '请选择 EchoKit 服务器' },
        ]}
      >
        <Select
          placeholder="选择 EchoKit 服务器"
          loading={loadingServers}
          notFoundContent={loadingServers ? <Spin size="small" /> : '暂无可用服务器'}
        >
          {echokitServers.map(server => (
            <Option key={server.id} value={server.server_url}>
              {server.server_url}
            </Option>
          ))}
        </Select>
      </Form.Item>

      <Form.Item>
        <Space>
          <Button type="primary" htmlType="submit" loading={loading}>
            开始注册
          </Button>
          <Button onClick={onClose}>
            取消
          </Button>
        </Space>
      </Form.Item>
    </Form>
  );

  // 渲染数据库确认
  const renderDatabaseConfirmation = () => (
    <div style={{ textAlign: 'center' }}>
      <CheckCircleOutlined style={{ fontSize: 64, color: '#52c41a', marginBottom: 24 }} />
      <Title level={3}>✅ 设备信息已成功写入数据库</Title>
      <Paragraph style={{ fontSize: 16, color: '#666', marginBottom: 32 }}>
        恭喜！您的设备信息已成功保存到数据库中。请确认以下信息无误后继续。
      </Paragraph>

      <Card style={{ marginBottom: 32, textAlign: 'left', maxWidth: 600, margin: '0 auto 32px' }}>
        <Descriptions column={1} title="设备信息确认" bordered>
          <Descriptions.Item label="设备ID">
            <Text code copyable>{createdDeviceInfo?.device_id}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="设备名称">
            {createdDeviceInfo?.name}
          </Descriptions.Item>
          <Descriptions.Item label="设备序列号">
            {createdDeviceInfo?.serial_number}
          </Descriptions.Item>
          <Descriptions.Item label="MAC地址">
            {createdDeviceInfo?.mac_address}
          </Descriptions.Item>
          <Descriptions.Item label="设备类型">
            {deviceTypeOptions.find(opt => opt.value === createdDeviceInfo?.device_type)?.label}
          </Descriptions.Item>
          <Descriptions.Item label="设备位置">
            {createdDeviceInfo?.location || '未设置'}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Space size="large">
        <Button size="large" onClick={() => setCurrentStep(0)}>
          <RollbackOutlined /> 重新填写
        </Button>
        <Button
          type="primary"
          size="large"
          onClick={handleDatabaseConfirmation}
          style={{ minWidth: 120 }}
        >
          确认无误，继续注册
        </Button>
      </Space>
    </div>
  );

  // 渲染带Tab的注册界面
  const renderTabbedRegistration = () => (
    <div>
      <Alert
        message="选择注册方式"
        description="请选择最适合您设备的注册方式完成配对"
        type="info"
        showIcon
        style={{ marginBottom: 24 }}
      />

      <Tabs
        activeKey={registrationMethod}
        onChange={handleTabChange}
        type="card"
        size="large"
        items={[
          {
            key: 'qr',
            label: (
              <span>
                <QrcodeOutlined />
                📱 二维码扫描（推荐）
              </span>
            ),
            children: renderQRRegistration(),
          },
          {
            key: 'manual',
            label: (
              <span>
                <KeyOutlined />
                ⌨️ 手动输入
              </span>
            ),
            children: renderManualRegistration(),
          },
        ]}
        style={{ marginBottom: 24 }}
      />

      <Space>
        <Button onClick={() => setCurrentStep(1)}>
          上一步
        </Button>
        <Button onClick={cancelRegistration} danger>
          取消注册
        </Button>
      </Space>
    </div>
  );

  // 渲染二维码注册
  const renderQRRegistration = () => (
    <div style={{ textAlign: 'center' }}>
      <Title level={4}>📱 二维码扫描注册</Title>
      <Paragraph>请在设备上扫描此二维码完成注册</Paragraph>

      <Card style={{ marginBottom: 24 }}>
        <QRCode
          value={registrationData?.qr_code_data || ''}
          size={200}
          style={{ marginBottom: 16 }}
        />
        <div>
          <Text strong>配对码：</Text>
          <Text code style={{ fontSize: 18, marginLeft: 8 }}>
            {registrationData?.pairing_code}
          </Text>
          <Tooltip title="复制配对码">
            <Button
              type="text"
              size="small"
              icon={<CopyOutlined />}
              onClick={() => copyToClipboard(registrationData?.pairing_code || '')}
              style={{ marginLeft: 8 }}
            />
          </Tooltip>
        </div>
      </Card>

      <Descriptions column={2} size="small" style={{ marginBottom: 24 }}>
        <Descriptions.Item label="设备名称">
          {createdDeviceInfo?.name}
        </Descriptions.Item>
        <Descriptions.Item label="设备类型">
          {deviceTypeOptions.find(opt => opt.value === createdDeviceInfo?.device_type)?.label}
        </Descriptions.Item>
        <Descriptions.Item label="设备位置">
          {createdDeviceInfo?.location || '未设置'}
        </Descriptions.Item>
        <Descriptions.Item label="剩余时间">
          <Tag color={timeLeft < 60 ? 'red' : 'green'}>
            {formatTime(timeLeft)}
          </Tag>
        </Descriptions.Item>
      </Descriptions>

      {timeLeft < 300 && (
        <Alert
          message="配对码即将过期"
          description={`配对码将在${formatTime(timeLeft)}后过期，请尽快完成注册`}
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Space>
        <Button onClick={regenerateRegistration} loading={loading} icon={<ReloadOutlined />}>
          重新生成
        </Button>
        {timeLeft < 300 && (
          <Button onClick={extendRegistration}>
            延长15分钟
          </Button>
        )}
        <Button onClick={() => setCurrentStep(0)}>
          返回重新选择
        </Button>
        <Button onClick={cancelRegistration} danger>
          取消注册
        </Button>
      </Space>
    </div>
  );

  // 渲染手动输入注册过程
  const renderManualRegistrationProcess = () => (
    <div style={{ textAlign: 'center' }}>
      <Title level={4}>⌨️ 手动输入注册</Title>
      <Paragraph>请在设备上手动输入以下配对码</Paragraph>

      <Card style={{ marginBottom: 24 }}>
        <div style={{ marginBottom: 16 }}>
          <Title level={2} code style={{ color: '#1890ff' }}>
            {registrationData?.pairing_code}
          </Title>
        </div>
        <Space>
          <Button
            type="primary"
            ghost
            icon={<CopyOutlined />}
            onClick={() => copyToClipboard(registrationData?.pairing_code || '')}
          >
            复制配对码
          </Button>
        </Space>
      </Card>

      <Card title="输入步骤" style={{ marginBottom: 24, textAlign: 'left' }}>
        <ol>
          <li>在设备上进入"设置" → "网络连接" → "添加账户"</li>
          <li>选择"通过配对码连接"</li>
          <li>输入配对码：<Text code>{registrationData?.pairing_code}</Text></li>
          <li>等待验证完成</li>
        </ol>
      </Card>

      <Descriptions column={2} size="small" style={{ marginBottom: 24 }}>
        <Descriptions.Item label="设备名称">
          {createdDeviceInfo?.name}
        </Descriptions.Item>
        <Descriptions.Item label="设备类型">
          {deviceTypeOptions.find(opt => opt.value === createdDeviceInfo?.device_type)?.label}
        </Descriptions.Item>
        <Descriptions.Item label="设备位置">
          {createdDeviceInfo?.location || '未设置'}
        </Descriptions.Item>
        <Descriptions.Item label="剩余时间">
          <Tag color={timeLeft < 60 ? 'red' : 'green'}>
            {formatTime(timeLeft)}
          </Tag>
        </Descriptions.Item>
      </Descriptions>

      {timeLeft < 300 && (
        <Alert
          message="配对码即将过期"
          description={`配对码将在${formatTime(timeLeft)}后过期，请尽快完成注册`}
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Space>
        <Button onClick={regenerateRegistration} loading={loading} icon={<ReloadOutlined />}>
          重新生成
        </Button>
        {timeLeft < 300 && (
          <Button onClick={extendRegistration}>
            延长15分钟
          </Button>
        )}
        <Button onClick={() => setCurrentStep(0)}>
          返回重新选择
        </Button>
        <Button onClick={cancelRegistration} danger>
          取消注册
        </Button>
      </Space>
    </div>
  );

  // 渲染注册成功
  const renderRegistrationSuccess = () => {
    console.log('渲染注册成功页面，设备信息:', createdDeviceInfo);
    return (
      <div style={{ textAlign: 'center' }}>
        <CheckCircleOutlined style={{ fontSize: 64, color: '#52c41a', marginBottom: 24 }} />
        <Title level={3}>🎉 设备注册成功！</Title>
        <Paragraph>
          设备 <Text strong>{createdDeviceInfo?.name}</Text> 已成功添加到您的账户
        </Paragraph>

      <Card style={{ marginBottom: 24, textAlign: 'left' }}>
        <Descriptions column={2} title="设备信息">
          <Descriptions.Item label="设备名称">
            {createdDeviceInfo?.name}
          </Descriptions.Item>
          <Descriptions.Item label="设备类型">
            {deviceTypeOptions.find(opt => opt.value === createdDeviceInfo?.device_type)?.label}
          </Descriptions.Item>
          <Descriptions.Item label="设备ID">
            <Text code copyable>{createdDeviceInfo?.device_id}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="序列号">
            {createdDeviceInfo?.serial_number}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Space>
        <Button type="primary" onClick={() => {
          onSuccess(); // 调用回调刷新设备列表
          onClose();   // 关闭模态框
        }}>
          确认
        </Button>
      </Space>
    </div>
    );
  };

  // 渲染注册Tab界面 - 模态窗口的主要内容
  const renderRegistrationTabs = () => (
    <Tabs
      activeKey={registrationMethod}
      onChange={handleTabChange}
      type="card"
      size="large"
      items={[
        {
          key: 'scan',
          label: (
            <span>
              <QrcodeOutlined />
              📱 扫码注册
            </span>
          ),
          children: renderScanRegistrationTab(),
        },
        {
          key: 'manual',
          label: (
            <span>
              <KeyOutlined />
              ⌨️ 手动注册
            </span>
          ),
          children: renderManualRegistrationTab(),
        },
      ]}
    />
  );

  // 渲染扫码注册Tab
  const renderScanRegistrationTab = () => (
    <div>
      <Alert
        message="扫码注册流程"
        description="使用设备的摄像头扫描二维码完成设备注册"
        type="info"
        showIcon
        style={{ marginBottom: 24 }}
      />

      <Card title="扫码注册步骤" style={{ marginBottom: 24 }}>
        <ol>
          <li>点击下方"开始扫码注册"按钮</li>
          <li>填写设备基本信息（设备名称、序列号、MAC地址等）</li>
          <li>系统生成注册二维码</li>
          <li>使用设备摄像头扫描二维码</li>
          <li>等待设备连接验证</li>
          <li>注册完成</li>
        </ol>
      </Card>

      <div style={{ textAlign: 'center' }}>
        <Space>
          <Button
            type="primary"
            size="large"
            onClick={() => {
              setCurrentStep(1); // 跳转到设备信息填写
              setRegistrationMethod('scan');
            }}
          >
            开始扫码注册
          </Button>
          <Button onClick={onClose}>
            取消
          </Button>
        </Space>
      </div>
    </div>
  );

  // 渲染手动注册Tab
  const renderManualRegistrationTab = () => (
    <div>
      <Alert
        message="手动注册流程"
        description="通过手动输入配对码完成设备注册"
        type="info"
        showIcon
        style={{ marginBottom: 24 }}
      />

      <Card title="手动注册步骤" style={{ marginBottom: 24 }}>
        <ol>
          <li>点击下方"开始手动注册"按钮</li>
          <li>填写设备基本信息（设备名称、序列号、MAC地址等）</li>
          <li>系统生成配对码</li>
          <li>在设备上手动输入配对码</li>
          <li>等待设备连接验证</li>
          <li>注册完成</li>
        </ol>
      </Card>

      <div style={{ textAlign: 'center' }}>
        <Space>
          <Button
            type="primary"
            size="large"
            onClick={() => {
              setCurrentStep(1); // 跳转到设备信息填写
              setRegistrationMethod('manual');
            }}
          >
            开始手动注册
          </Button>
          <Button onClick={onClose}>
            取消
          </Button>
        </Space>
      </div>
    </div>
  );

  // 渲染当前步骤内容
  const renderStepContent = () => {
    console.log('渲染步骤内容，当前步骤:', currentStep);
    switch (currentStep) {
      case 0:
        return renderRegistrationTabs();
      case 1:
        return renderDeviceInfoForm();
      case 2:
        return renderDatabaseConfirmation();
      case 3:
        return registrationMethod === 'scan' ? renderQRRegistration() : renderManualRegistrationProcess();
      case 4:
        return renderRegistrationSuccess();
      default:
        return renderRegistrationTabs();
    }
  };

  return (
    <Modal
      title="注册新设备"
      open={visible}
      onCancel={onClose}
      footer={null}
      width={800}
    >
      {loading && (
        <div style={{ textAlign: 'center', padding: 20 }}>
          <Spin size="large" />
        </div>
      )}

      {!loading && renderStepContent()}
    </Modal>
  );
};

export default DeviceRegistrationModal;
import React, { useState, useEffect } from 'react';
import { echokitServersApi, EchoKitServer as APIEchoKitServer } from '../api/echokitServers';
import {
  Card,
  Tabs,
  Form,
  Input,
  Button,
  Switch,
  Select,
  InputNumber,
  Space,
  Divider,
  message,
  Alert,
  Row,
  Col,
  Statistic,
  Table,
  Badge,
  Tooltip,
  Modal,
  Popconfirm
} from 'antd';
import {
  SettingOutlined,
  SecurityScanOutlined,
  BellOutlined,
  ApiOutlined,
  SaveOutlined,
  ReloadOutlined,
  ExclamationCircleOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  SyncOutlined,
  PlusOutlined,
  DeleteOutlined,
  ThunderboltOutlined
} from '@ant-design/icons';
import { Resizable } from 'react-resizable';
import type { ResizeCallbackData } from 'react-resizable';
import '../styles/resizable.css';

const { TabPane } = Tabs;
const { TextArea } = Input;

interface EchoKitServer {
  id: string;
  url: string;
  status: 'available' | 'unavailable' | 'unknown';
  lastChecked: string;
  checking?: boolean;
}

// ResizableTitle component for column headers
const ResizableTitle = (props: any) => {
  const { onResize, width, ...restProps } = props;

  if (!width) {
    return <th {...restProps} />;
  }

  return (
    <Resizable
      width={width}
      height={0}
      handle={
        <span
          className="react-resizable-handle"
          onClick={(e) => e.stopPropagation()}
        />
      }
      onResize={onResize}
      draggableOpts={{ enableUserSelectHack: false }}
    >
      <th {...restProps} />
    </Resizable>
  );
};

export const Settings: React.FC = () => {
  const [systemForm] = Form.useForm();
  const [notificationForm] = Form.useForm();
  const [loading, setLoading] = useState(false);

  // EchoKit Server 列表状态
  const [echokitServers, setEchokitServers] = useState<EchoKitServer[]>([]);

  // 加载服务器列表
  useEffect(() => {
    loadServers();
  }, []);

  const loadServers = async () => {
    try {
      const servers = await echokitServersApi.getServers();
      // 转换 API 数据格式到 UI 格式，初始状态为 unknown
      const uiServers = servers.map(server => ({
        id: server.id.toString(),
        url: server.server_url,
        status: 'unknown' as const,
        lastChecked: '未测试',
        checking: false
      }));
      setEchokitServers(uiServers);

      // 自动逐个测试所有服务器
      testAllServersSequentially(uiServers);
    } catch (error) {
      console.error('Failed to load EchoKit servers:', error);
      message.error('加载服务器列表失败');
    }
  };

  // 列宽状态
  const [columns, setColumns] = useState([
    { key: 'url', width: 350 },
    { key: 'status', width: 130 },
    { key: 'lastChecked', width: 200 },
    { key: 'action', width: 150 }
  ]);

  // 处理列宽调整
  const handleResize = (key: string) => (_: React.SyntheticEvent, { size }: ResizeCallbackData) => {
    setColumns((prevColumns) =>
      prevColumns.map((col) =>
        col.key === key ? { ...col, width: size.width } : col
      )
    );
  };

  // 添加服务器 Modal 状态
  const [isAddModalVisible, setIsAddModalVisible] = useState(false);
  const [addServerForm] = Form.useForm();
  const [isTestingConnection, setIsTestingConnection] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  // 模拟系统信息
  const systemInfo = {
    version: '1.0.0',
    buildTime: '2024-10-24',
    uptime: '2天 3小时 45分钟',
    totalDevices: 156,
    activeUsers: 23
  };

  // 保存系统设置
  const handleSaveSystem = async (values: any) => {
    setLoading(true);
    try {
      // 模拟API调用
      await new Promise(resolve => setTimeout(resolve, 1000));
      message.success('系统设置保存成功');
    } catch (error) {
      message.error('保存失败');
    } finally {
      setLoading(false);
    }
  };

  // 保存通知设置
  const handleSaveNotification = async (values: any) => {
    setLoading(true);
    try {
      await new Promise(resolve => setTimeout(resolve, 1000));
      message.success('通知设置保存成功');
    } catch (error) {
      message.error('保存失败');
    } finally {
      setLoading(false);
    }
  };

  // 刷新单个 EchoKit Server 状态
  const handleRefreshServer = async (serverId: string, serverUrl?: string) => {
    setEchokitServers(prev =>
      prev.map(server =>
        server.id === serverId ? { ...server, checking: true } : server
      )
    );

    try {
      // 如果传入了 serverUrl 就使用它,否则从状态中查找
      let url = serverUrl;
      if (!url) {
        const server = echokitServers.find(s => s.id === serverId);
        if (!server) {
          throw new Error('Server not found');
        }
        url = server.url;
      }

      // 测试连接
      const testUrl = url.endsWith('/') ? url : url + '/';
      const fullTestUrl = testUrl + 'test_' + Date.now();

      let isAvailable = false;
      try {
        await new Promise<void>((resolve, reject) => {
          const testTimeout = setTimeout(() => {
            reject(new Error('连接超时'));
          }, 5000);

          try {
            const ws = new WebSocket(fullTestUrl);

            ws.onopen = () => {
              clearTimeout(testTimeout);
              ws.close();
              isAvailable = true;
              resolve();
            };

            ws.onerror = () => {
              clearTimeout(testTimeout);
              reject(new Error('无法连接'));
            };
          } catch (err) {
            clearTimeout(testTimeout);
            reject(err);
          }
        });
      } catch (err) {
        isAvailable = false;
      }

      // 只更新前端状态，不调用后端 API
      setEchokitServers(prev =>
        prev.map(s =>
          s.id === serverId
            ? {
                ...s,
                status: isAvailable ? 'available' : 'unavailable',
                lastChecked: new Date().toLocaleString('zh-CN'),
                checking: false
              }
            : s
        )
      );
    } catch (error) {
      console.error('Failed to refresh server:', error);
      setEchokitServers(prev =>
        prev.map(server =>
          server.id === serverId ? { ...server, checking: false } : server
        )
      );
      message.error('刷新失败');
    }
  };

  // 自动逐个测试所有服务器
  const testAllServersSequentially = async (servers: EchoKitServer[]) => {
    for (const server of servers) {
      await handleRefreshServer(server.id, server.url);
      // 添加短暂延迟避免并发过多
      await new Promise(resolve => setTimeout(resolve, 200));
    }
  };

  // 刷新所有 EchoKit Server 状态
  const handleRefreshAllServers = async () => {
    setEchokitServers(prev => prev.map(server => ({ ...server, checking: true })));

    try {
      // 模拟API调用
      await new Promise(resolve => setTimeout(resolve, 2000));

      setEchokitServers(prev =>
        prev.map(server => ({
          ...server,
          status: Math.random() > 0.3 ? 'available' : 'unavailable',
          lastChecked: new Date().toLocaleString('zh-CN'),
          checking: false
        }))
      );

      message.success('所有服务器状态已更新');
    } catch (error) {
      setEchokitServers(prev => prev.map(server => ({ ...server, checking: false })));
      message.error('刷新失败');
    }
  };

  // 测试服务器连接
  const handleTestConnection = async () => {
    const url = addServerForm.getFieldValue('url');
    if (!url) {
      message.warning('请先输入服务器URL');
      return;
    }

    setIsTestingConnection(true);
    setTestResult(null);

    // 创建一个临时的 WebSocket 连接来测试
    let ws: WebSocket | null = null;
    let testTimeout: ReturnType<typeof setTimeout>;

    try {
      // 构建完整的 WebSocket URL (添加一个临时路径用于测试)
      const testUrl = url.endsWith('/') ? url : url + '/';
      const fullTestUrl = testUrl + 'test_' + Date.now();

      await new Promise<void>((resolve, reject) => {
        // 设置 5 秒超时
        testTimeout = setTimeout(() => {
          if (ws) {
            ws.close();
          }
          reject(new Error('连接超时（5秒）'));
        }, 5000);

        try {
          ws = new WebSocket(fullTestUrl);

          ws.onopen = () => {
            clearTimeout(testTimeout);
            if (ws) {
              ws.close();
            }
            resolve();
          };

          ws.onerror = () => {
            clearTimeout(testTimeout);
            reject(new Error('无法连接到服务器'));
          };

          ws.onclose = (event) => {
            clearTimeout(testTimeout);
            // 如果是正常关闭（我们主动关闭的），说明连接成功
            if (event.code === 1000) {
              resolve();
            }
          };
        } catch (err) {
          clearTimeout(testTimeout);
          reject(err);
        }
      });

      setTestResult({
        success: true,
        message: '连接成功！服务器可用'
      });
      message.success('测试连接成功');

    } catch (error: any) {
      const errorMessage = error.message || '连接失败，请检查URL是否正确';
      setTestResult({
        success: false,
        message: errorMessage
      });
      message.error(`测试连接失败: ${errorMessage}`);
    } finally {
      setIsTestingConnection(false);
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    }
  };

  // 添加服务器
  const handleAddServer = async (values: { url: string }) => {
    try {
      // 调用后端 API 添加服务器
      const newServer = await echokitServersApi.addServer({
        server_url: values.url
      });

      // 转换为 UI 格式并添加到列表
      const uiServer: EchoKitServer = {
        id: newServer.id.toString(),
        url: newServer.server_url,
        status: newServer.status as 'available' | 'unavailable',
        lastChecked: newServer.last_checked_at
          ? new Date(newServer.last_checked_at).toLocaleString('zh-CN')
          : '未检查',
        checking: false
      };

      setEchokitServers(prev => [...prev, uiServer]);
      message.success('服务器添加成功');
      setIsAddModalVisible(false);
      addServerForm.resetFields();
      setTestResult(null);

      // 添加成功后自动刷新状态,传递 URL 避免状态更新延迟问题
      setTimeout(() => {
        handleRefreshServer(newServer.id.toString(), newServer.server_url);
      }, 100);
    } catch (error: any) {
      console.error('Failed to add server:', error);
      if (error.response?.status === 409) {
        message.error('该服务器URL已存在');
      } else {
        message.error('添加服务器失败');
      }
    }
  };

  // 删除服务器
  const handleDeleteServer = async (serverId: string) => {
    try {
      await echokitServersApi.deleteServer(parseInt(serverId));
      setEchokitServers(prev => prev.filter(server => server.id !== serverId));
      message.success('服务器已删除');
    } catch (error) {
      console.error('Failed to delete server:', error);
      message.error('删除服务器失败');
    }
  };

  // 打开添加服务器对话框
  const showAddModal = () => {
    setIsAddModalVisible(true);
    setTestResult(null);
  };

  // 关闭添加服务器对话框
  const handleCancelAddModal = () => {
    setIsAddModalVisible(false);
    addServerForm.resetFields();
    setTestResult(null);
  };

  return (
    <div style={{ padding: 24 }}>
      <div style={{ marginBottom: 24 }}>
        <h2>系统设置</h2>
        <p style={{ color: '#8c8c8c' }}>配置系统参数、通知设置和API集成</p>
      </div>

      <Tabs defaultActiveKey="system">
        {/* 系统设置 */}
        <TabPane tab={<span><SettingOutlined />系统设置</span>} key="system">
          <Row gutter={[16, 16]}>
            <Col xs={24} lg={16}>
              <Card title="基本配置">
                <Form
                  form={systemForm}
                  layout="vertical"
                  onFinish={handleSaveSystem}
                  initialValues={{
                    siteName: 'Echo Web Management',
                    maxDevices: 1000,
                    sessionTimeout: 30,
                    autoBackup: true,
                    logLevel: 'info'
                  }}
                >
                  <Form.Item
                    name="siteName"
                    label="系统名称"
                    rules={[{ required: true, message: '请输入系统名称' }]}
                  >
                    <Input placeholder="请输入系统名称" />
                  </Form.Item>

                  <Form.Item
                    name="maxDevices"
                    label="最大设备数量"
                  >
                    <InputNumber
                      min={1}
                      max={10000}
                      style={{ width: '100%' }}
                      addonAfter="台"
                    />
                  </Form.Item>

                  <Form.Item
                    name="sessionTimeout"
                    label="会话超时时间"
                  >
                    <InputNumber
                      min={1}
                      max={120}
                      style={{ width: '100%' }}
                      addonAfter="分钟"
                    />
                  </Form.Item>

                  <Form.Item
                    name="logLevel"
                    label="日志级别"
                  >
                    <Select>
                      <Select.Option value="debug">Debug</Select.Option>
                      <Select.Option value="info">Info</Select.Option>
                      <Select.Option value="warning">Warning</Select.Option>
                      <Select.Option value="error">Error</Select.Option>
                    </Select>
                  </Form.Item>

                  <Form.Item
                    name="autoBackup"
                    label="自动备份"
                    valuePropName="checked"
                  >
                    <Switch />
                  </Form.Item>

                  <Divider />

                  <Form.Item>
                    <Space>
                      <Button
                        type="primary"
                        htmlType="submit"
                        icon={<SaveOutlined />}
                        loading={loading}
                      >
                        保存设置
                      </Button>
                      <Button icon={<ReloadOutlined />}>
                        重置
                      </Button>
                    </Space>
                  </Form.Item>
                </Form>
              </Card>
            </Col>

            <Col xs={24} lg={8}>
              <Card title="系统信息">
                <Space direction="vertical" style={{ width: '100%' }}>
                  <Statistic
                    title="系统版本"
                    value={systemInfo.version}
                    prefix={<ApiOutlined />}
                  />
                  <Statistic
                    title="构建时间"
                    value={systemInfo.buildTime}
                  />
                  <Statistic
                    title="运行时间"
                    value={systemInfo.uptime}
                  />
                  <Statistic
                    title="设备总数"
                    value={systemInfo.totalDevices}
                  />
                  <Statistic
                    title="活跃用户"
                    value={systemInfo.activeUsers}
                  />
                </Space>
              </Card>

              <Card title="系统状态" style={{ marginTop: 16 }}>
                <Alert
                  message="系统运行正常"
                  description="所有服务运行正常，系统性能良好"
                  type="success"
                  showIcon
                />
              </Card>
            </Col>
          </Row>
        </TabPane>

        {/* 通知设置 */}
        <TabPane tab={<span><BellOutlined />通知设置</span>} key="notification">
          <Card title="通知配置">
            <Form
              form={notificationForm}
              layout="vertical"
              onFinish={handleSaveNotification}
              initialValues={{
                emailEnabled: true,
                emailSmtp: 'smtp.example.com',
                emailPort: 587,
                emailUsername: 'admin@example.com',
                smsEnabled: false,
                webhookEnabled: true,
                webhookUrl: 'https://api.example.com/webhook'
              }}
            >
              <h4>邮件通知</h4>
              <Form.Item
                name="emailEnabled"
                label="启用邮件通知"
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>

              <Form.Item
                name="emailSmtp"
                label="SMTP服务器"
                rules={[{ required: true, message: '请输入SMTP服务器' }]}
              >
                <Input placeholder="smtp.example.com" />
              </Form.Item>

              <Form.Item
                name="emailPort"
                label="SMTP端口"
                rules={[{ required: true, message: '请输入SMTP端口' }]}
              >
                <InputNumber
                  min={1}
                  max={65535}
                  style={{ width: '100%' }}
                />
              </Form.Item>

              <Form.Item
                name="emailUsername"
                label="邮箱用户名"
                rules={[{ required: true, message: '请输入邮箱用户名' }]}
              >
                <Input placeholder="admin@example.com" />
              </Form.Item>

              <Form.Item
                name="emailPassword"
                label="邮箱密码"
              >
                <Input.Password placeholder="请输入邮箱密码" />
              </Form.Item>

              <Divider />

              <h4>短信通知</h4>
              <Form.Item
                name="smsEnabled"
                label="启用短信通知"
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>

              <Form.Item
                name="smsApiKey"
                label="短信API Key"
              >
                <Input placeholder="请输入短信API Key" />
              </Form.Item>

              <Form.Item
                name="smsTemplate"
                label="短信模板"
              >
                <TextArea
                  rows={3}
                  placeholder="设备【{deviceName}】状态变更为【{status}】"
                />
              </Form.Item>

              <Divider />

              <h4>Webhook通知</h4>
              <Form.Item
                name="webhookEnabled"
                label="启用Webhook通知"
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>

              <Form.Item
                name="webhookUrl"
                label="Webhook URL"
                rules={[{ required: true, message: '请输入Webhook URL' }]}
              >
                <Input placeholder="https://api.example.com/webhook" />
              </Form.Item>

              <Form.Item>
                <Button
                  type="primary"
                  htmlType="submit"
                  icon={<SaveOutlined />}
                  loading={loading}
                >
                  保存设置
                </Button>
              </Form.Item>
            </Form>
          </Card>
        </TabPane>

        {/* EchoKit Server 设置 */}
        <TabPane tab={<span><ApiOutlined />EchoKit Server 设置</span>} key="echokit">
          <Card
            title="EchoKit Server 列表"
            extra={
              <Space>
                <Button
                  type="primary"
                  icon={<PlusOutlined />}
                  onClick={showAddModal}
                >
                  添加服务器
                </Button>
                <Button
                  icon={<ReloadOutlined />}
                  onClick={handleRefreshAllServers}
                  loading={echokitServers.some(s => s.checking)}
                >
                  刷新所有
                </Button>
              </Space>
            }
          >
            <Alert
              message="EchoKit Server 说明"
              description="管理和监控 EchoKit Server 的连接状态。绿色表示服务可用，红色表示服务不可用。"
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
            />

            <Table
              dataSource={echokitServers}
              rowKey="id"
              pagination={false}
              bordered
              components={{
                header: {
                  cell: ResizableTitle,
                },
              }}
              columns={[
                {
                  title: 'Server URL',
                  dataIndex: 'url',
                  key: 'url',
                  ellipsis: true,
                  width: columns.find(c => c.key === 'url')?.width,
                  onHeaderCell: () => ({
                    width: columns.find(c => c.key === 'url')?.width,
                    onResize: handleResize('url'),
                  }),
                  render: (url: string) => (
                    <span style={{ fontFamily: 'monospace' }}>{url}</span>
                  )
                },
                {
                  title: '状态',
                  dataIndex: 'status',
                  key: 'status',
                  align: 'center' as const,
                  width: columns.find(c => c.key === 'status')?.width,
                  onHeaderCell: () => ({
                    width: columns.find(c => c.key === 'status')?.width,
                    onResize: handleResize('status'),
                  }),
                  render: (status: 'available' | 'unavailable' | 'unknown') => (
                    <Badge
                      status={status === 'available' ? 'success' : status === 'unavailable' ? 'error' : 'warning'}
                      text={
                        <span>
                          {status === 'available' ? (
                            <>
                              <CheckCircleOutlined style={{ color: '#52c41a', marginRight: 4 }} />
                              可用
                            </>
                          ) : status === 'unavailable' ? (
                            <>
                              <CloseCircleOutlined style={{ color: '#ff4d4f', marginRight: 4 }} />
                              不可用
                            </>
                          ) : (
                            <>
                              <ExclamationCircleOutlined style={{ color: '#faad14', marginRight: 4 }} />
                              未知
                            </>
                          )}
                        </span>
                      }
                    />
                  )
                },
                {
                  title: '最近检查时间',
                  dataIndex: 'lastChecked',
                  key: 'lastChecked',
                  width: columns.find(c => c.key === 'lastChecked')?.width,
                  onHeaderCell: () => ({
                    width: columns.find(c => c.key === 'lastChecked')?.width,
                    onResize: handleResize('lastChecked'),
                  }),
                  render: (time: string) => (
                    <Tooltip title="上次刷新状态的时间">
                      <span style={{ color: '#8c8c8c' }}>{time}</span>
                    </Tooltip>
                  )
                },
                {
                  title: '操作',
                  key: 'action',
                  align: 'center' as const,
                  width: columns.find(c => c.key === 'action')?.width,
                  render: (_: any, record: EchoKitServer) => (
                    <Space size="small">
                      <Button
                        type="link"
                        size="small"
                        icon={<SyncOutlined spin={record.checking} />}
                        onClick={() => handleRefreshServer(record.id)}
                        loading={record.checking}
                      >
                        刷新
                      </Button>
                      <Popconfirm
                        title="删除服务器"
                        description="确定要删除这个服务器吗？"
                        onConfirm={() => handleDeleteServer(record.id)}
                        okText="确定"
                        cancelText="取消"
                      >
                        <Button
                          type="link"
                          size="small"
                          danger
                          icon={<DeleteOutlined />}
                        >
                          删除
                        </Button>
                      </Popconfirm>
                    </Space>
                  )
                }
              ]}
            />
          </Card>

          {/* 添加服务器 Modal */}
          <Modal
            title="添加 EchoKit Server"
            open={isAddModalVisible}
            onCancel={handleCancelAddModal}
            footer={[
              <Button key="cancel" onClick={handleCancelAddModal}>
                取消
              </Button>,
              <Button
                key="test"
                icon={<ThunderboltOutlined />}
                onClick={handleTestConnection}
                loading={isTestingConnection}
              >
                测试连接
              </Button>,
              <Button
                key="submit"
                type="primary"
                onClick={() => addServerForm.submit()}
                disabled={!testResult?.success}
              >
                添加
              </Button>,
            ]}
          >
            <Form
              form={addServerForm}
              layout="vertical"
              onFinish={handleAddServer}
            >
              <Form.Item
                name="url"
                label="服务器 URL"
                rules={[
                  { required: true, message: '请输入服务器URL' },
                  {
                    pattern: /^wss?:\/\/.+/,
                    message: '请输入有效的 WebSocket URL (ws:// 或 wss://)'
                  }
                ]}
              >
                <Input
                  placeholder="例如: wss://indie.echokit.dev/ws"
                  onChange={() => setTestResult(null)}
                />
              </Form.Item>

              {testResult && (
                <Alert
                  message={testResult.success ? '连接测试成功' : '连接测试失败'}
                  description={testResult.message}
                  type={testResult.success ? 'success' : 'error'}
                  showIcon
                  style={{ marginBottom: 16 }}
                />
              )}

              <Alert
                message="提示"
                description="请先点击【测试连接】按钮验证服务器可用性，测试成功后才能添加服务器。"
                type="info"
                showIcon
              />
            </Form>
          </Modal>
        </TabPane>

        {/* 安全设置 */}
        <TabPane tab={<span><SecurityScanOutlined />安全设置</span>} key="security">
          <Card title="安全配置">
            <Alert
              message="安全警告"
              description="修改安全设置可能影响系统访问，请谨慎操作。"
              type="warning"
              showIcon
              icon={<ExclamationCircleOutlined />}
              style={{ marginBottom: 16 }}
            />

            <Form layout="vertical">
              <h4>访问控制</h4>
              <Form.Item label="启用双因素认证">
                <Switch />
              </Form.Item>

              <Form.Item label="强制HTTPS">
                <Switch defaultChecked />
              </Form.Item>

              <Form.Item label="会话超时时间">
                <InputNumber
                  min={5}
                  max={120}
                  defaultValue={30}
                  addonAfter="分钟"
                />
              </Form.Item>

              <Form.Item label="密码策略">
                <Space direction="vertical" style={{ width: '100%' }}>
                  <div>
                    <label>最小长度</label>
                    <InputNumber min={6} max={20} defaultValue={8} />
                  </div>
                  <div>
                    <label>包含特殊字符</label>
                    <Switch defaultChecked />
                  </div>
                  <div>
                    <label>包含数字</label>
                    <Switch defaultChecked />
                  </div>
                </Space>
              </Form.Item>

              <Divider />

              <h4>访问日志</h4>
              <Form.Item label="记录访问日志">
                <Switch defaultChecked />
              </Form.Item>

              <Form.Item label="日志保留时间">
                <Select defaultValue="30">
                  <Select.Option value="7">7天</Select.Option>
                  <Select.Option value="30">30天</Select.Option>
                  <Select.Option value="90">90天</Select.Option>
                  <Select.Option value="365">1年</Select.Option>
                </Select>
              </Form.Item>

              <Form.Item>
                <Button type="primary" icon={<SaveOutlined />}>
                  保存安全设置
                </Button>
              </Form.Item>
            </Form>
          </Card>
        </TabPane>
      </Tabs>
    </div>
  );
};
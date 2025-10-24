import React, { useState } from 'react';
import {
  Card,
  Tabs,
  Form,
  Input,
  Button,
  Switch,
  Select,
  Slider,
  InputNumber,
  Space,
  Divider,
  message,
  Alert,
  Row,
  Col,
  Statistic
} from 'antd';
import {
  SettingOutlined,
  SecurityScanOutlined,
  BellOutlined,
  DatabaseOutlined,
  ApiOutlined,
  SaveOutlined,
  ReloadOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';

const { TabPane } = Tabs;
const { TextArea } = Input;

export const Settings: React.FC = () => {
  const [systemForm] = Form.useForm();
  const [notificationForm] = Form.useForm();
  const [apiForm] = Form.useForm();
  const [loading, setLoading] = useState(false);

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

  // 保存API设置
  const handleSaveApi = async (values: any) => {
    setLoading(true);
    try {
      await new Promise(resolve => setTimeout(resolve, 1000));
      message.success('API设置保存成功');
    } catch (error) {
      message.error('保存失败');
    } finally {
      setLoading(false);
    }
  };

  // 测试API连接
  const handleTestApi = async () => {
    try {
      await new Promise(resolve => setTimeout(resolve, 1000));
      message.success('API连接测试成功');
    } catch (error) {
      message.error('API连接测试失败');
    }
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

        {/* API设置 */}
        <TabPane tab={<span><ApiOutlined />API设置</span>} key="api">
          <Card title="API配置">
            <Alert
              message="API配置说明"
              description="配置第三方服务API密钥，用于语音识别、语音合成和智能对话功能。"
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
            />

            <Form
              form={apiForm}
              layout="vertical"
              onFinish={handleSaveApi}
              initialValues={{
                asrProvider: 'openai',
                llmProvider: 'openai',
                ttsProvider: 'azure',
                openaiApiKey: '',
                azureApiKey: '',
                azureRegion: 'eastus'
              }}
            >
              <h4>语音识别（ASR）</h4>
              <Form.Item
                name="asrProvider"
                label="ASR服务商"
              >
                <Select>
                  <Select.Option value="openai">OpenAI Whisper</Select.Option>
                  <Select.Option value="azure">Azure Speech</Select.Option>
                  <Select.Option value="google">Google Speech</Select.Option>
                  <Select.Option value="baidu">百度语音</Select.Option>
                </Select>
              </Form.Item>

              <Form.Item
                name="asrApiKey"
                label="ASR API Key"
              >
                <Input.Password placeholder="请输入ASR API Key" />
              </Form.Item>

              <Divider />

              <h4>大语言模型（LLM）</h4>
              <Form.Item
                name="llmProvider"
                label="LLM服务商"
              >
                <Select>
                  <Select.Option value="openai">OpenAI GPT</Select.Option>
                  <Select.Option value="azure">Azure OpenAI</Select.Option>
                  <Select.Option value="anthropic">Anthropic Claude</Select.Option>
                  <Select.Option value="google">Google Gemini</Select.Option>
                </Select>
              </Form.Item>

              <Form.Item
                name="llmApiKey"
                label="LLM API Key"
              >
                <Input.Password placeholder="请输入LLM API Key" />
              </Form.Item>

              <Form.Item
                name="llmModel"
                label="模型选择"
              >
                <Select>
                  <Select.Option value="gpt-4">GPT-4</Select.Option>
                  <Select.Option value="gpt-3.5-turbo">GPT-3.5 Turbo</Select.Option>
                  <Select.Option value="claude-3">Claude 3</Select.Option>
                  <Select.Option value="gemini-pro">Gemini Pro</Select.Option>
                </Select>
              </Form.Item>

              <Form.Item
                name="temperature"
                label="创造性（Temperature）"
              >
                <Slider
                  min={0}
                  max={2}
                  step={0.1}
                  marks={{
                    0: '保守',
                    1: '平衡',
                    2: '创造'
                  }}
                />
              </Form.Item>

              <Divider />

              <h4>语音合成（TTS）</h4>
              <Form.Item
                name="ttsProvider"
                label="TTS服务商"
              >
                <Select>
                  <Select.Option value="azure">Azure Speech</Select.Option>
                  <Select.Option value="google">Google TTS</Select.Option>
                  <Select.Option value="amazon">Amazon Polly</Select.Option>
                  <Select.Option value="azure-neural">Azure Neural</Select.Option>
                </Select>
              </Form.Item>

              <Form.Item
                name="ttsApiKey"
                label="TTS API Key"
              >
                <Input.Password placeholder="请输入TTS API Key" />
              </Form.Item>

              <Form.Item
                name="ttsVoice"
                label="语音选择"
              >
                <Select>
                  <Select.Option value="zh-CN-XiaoxiaoNeural">晓晓（女声）</Select.Option>
                  <Select.Option value="zh-CN-YunxiNeural">云希（男声）</Select.Option>
                  <Select.Option value="zh-CN-YunyangNeural">云扬（男声）</Select.Option>
                </Select>
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
                  <Button icon={<ApiOutlined />} onClick={handleTestApi}>
                    测试连接
                  </Button>
                </Space>
              </Form.Item>
            </Form>
          </Card>
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
import { ConfigProvider, Button, Card, Input, Form, Layout, Menu, Avatar, Row, Col, Statistic, Table, Tag, List, Timeline, Switch, InputNumber, Select } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { UserOutlined, LockOutlined, DashboardOutlined, AudioOutlined, HistoryOutlined, SettingOutlined, MenuFoldOutlined, MenuUnfoldOutlined, PlayCircleOutlined, WifiOutlined, ReloadOutlined } from '@ant-design/icons';
import { useState } from 'react';
import './App.css';

const { Header, Sider, Content } = Layout;

// 登录组件
const LoginPage = ({ onLogin }: { onLogin: () => void }) => {
  const [loading, setLoading] = useState(false);

  const handleLogin = async (values: any) => {
    setLoading(true);
    // 模拟登录验证
    setTimeout(() => {
      setLoading(false);
      if (values.username === 'admin' && values.password === 'admin123') {
        onLogin();
      } else {
        alert('用户名或密码错误！');
      }
    }, 1000);
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        padding: 20
      }}
    >
      <Card
        style={{
          width: '100%',
          maxWidth: 400,
          boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1)'
        }}
        bodyStyle={{ padding: 40 }}
      >
        {/* Logo和标题 */}
        <div style={{ textAlign: 'center', marginBottom: 32 }}>
          <div
            style={{
              width: 64,
              height: 64,
              borderRadius: '50%',
              background: '#1890ff',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              margin: '0 auto 16px',
              color: 'white',
              fontSize: 24
            }}
          >
            🔊
          </div>
          <h2 style={{ margin: 0, color: '#262626' }}>Echo Web</h2>
          <p style={{ margin: '8px 0 0', color: '#8c8c8c' }}>
            智能音箱管理平台
          </p>
        </div>

        {/* 登录表单 */}
        <Form
          name="login"
          onFinish={handleLogin}
          autoComplete="off"
          size="large"
        >
          <Form.Item
            name="username"
            rules={[{ required: true, message: '请输入用户名' }]}
          >
            <Input
              prefix={<UserOutlined />}
              placeholder="用户名"
            />
          </Form.Item>

          <Form.Item
            name="password"
            rules={[{ required: true, message: '请输入密码' }]}
          >
            <Input.Password
              prefix={<LockOutlined />}
              placeholder="密码"
            />
          </Form.Item>

          <Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              loading={loading}
              style={{ width: '100%' }}
            >
              登录
            </Button>
          </Form.Item>
        </Form>

        {/* 演示账户信息 */}
        <div
          style={{
            background: '#f6f8fa',
            padding: 16,
            borderRadius: 6,
            fontSize: 12,
            marginTop: 16
          }}
        >
          <div style={{ marginBottom: 8, fontWeight: 500 }}>测试账户：</div>
          <div>用户名：admin</div>
          <div>密码：admin123</div>
        </div>
      </Card>
    </div>
  );
};

// 仪表板组件
const DashboardPage = () => {
  return (
    <div style={{ padding: 24 }}>
      <div style={{ marginBottom: 24 }}>
        <h2>仪表板</h2>
        <p style={{ color: '#8c8c8c' }}>智能音箱设备管理概览</p>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="设备总数"
              value={3}
              prefix={<AudioOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="在线设备"
              value={2}
              valueStyle={{ color: '#3f8600' }}
              prefix={<WifiOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="活跃会话"
              value={1}
              valueStyle={{ color: '#1890ff' }}
              prefix={<PlayCircleOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="今日会话"
              value={15}
              prefix={<HistoryOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
        <Col xs={24} lg={12}>
          <Card title="设备状态">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>🔊 客厅音箱</span>
                <Tag color="success">在线</Tag>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>📱 卧室显示屏</span>
                <Tag>离线</Tag>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>🎛️ 厨房中控</span>
                <Tag color="success">在线</Tag>
              </div>
            </div>
          </Card>
        </Col>

        <Col xs={24} lg={12}>
          <Card title="系统状态">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>WebSocket连接</span>
                <span style={{ color: '#52c41a' }}>● 已连接</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>数据同步</span>
                <span style={{ color: '#52c41a' }}>● 正常</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>系统运行时间</span>
                <span>2小时15分钟</span>
              </div>
            </div>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

// 设备管理组件
const DevicesPage = () => {
  const [devices] = useState([
    {
      id: 'SPEAKER001',
      name: '客厅音箱',
      type: '智能音箱',
      location: '客厅',
      status: 'online',
      battery: 85,
      volume: 60,
      firmware: '1.2.3'
    },
    {
      id: 'DISPLAY001',
      name: '卧室显示屏',
      type: '智能显示屏',
      location: '主卧室',
      status: 'offline',
      battery: 45,
      volume: 30,
      firmware: '1.2.2'
    },
    {
      id: 'HUB001',
      name: '厨房中控',
      type: '中控设备',
      location: '厨房',
      status: 'online',
      battery: 92,
      volume: 40,
      firmware: '1.2.3'
    }
  ]);

  const getStatusColor = (status: string) => {
    return status === 'online' ? 'success' : 'default';
  };

  const getStatusText = (status: string) => {
    return status === 'online' ? '在线' : '离线';
  };

  const handleRestart = (deviceId: string) => {
    alert(`重启设备: ${deviceId}`);
  };

  const handleConfigure = (deviceId: string) => {
    alert(`配置设备: ${deviceId}`);
  };

  return (
    <div style={{ padding: 24 }}>
      <div style={{ marginBottom: 24 }}>
        <h2>设备管理</h2>
        <p style={{ color: '#8c8c8c' }}>管理和配置您的智能音箱设备</p>
      </div>

      <Row gutter={[16, 16]}>
        {devices.map((device) => (
          <Col xs={24} sm={12} lg={8} key={device.id}>
            <Card
              title={
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span>
                    {device.type === '智能音箱' ? '🔊' :
                     device.type === '智能显示屏' ? '📱' : '🎛️'}
                  </span>
                  {device.name}
                </div>
              }
              extra={
                <Tag color={getStatusColor(device.status)}>
                  {getStatusText(device.status)}
                </Tag>
              }
            >
              <div style={{ marginBottom: 16 }}>
                <p><strong>设备ID:</strong> {device.id}</p>
                <p><strong>位置:</strong> {device.location}</p>
                <p><strong>固件版本:</strong> {device.firmware}</p>
                <p><strong>电量:</strong> {device.battery}%</p>
                <p><strong>音量:</strong> {device.volume}%</p>
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <Button
                  type="primary"
                  size="small"
                  onClick={() => handleConfigure(device.id)}
                >
                  配置
                </Button>
                <Button
                  size="small"
                  onClick={() => handleRestart(device.id)}
                  disabled={device.status !== 'online'}
                >
                  重启
                </Button>
              </div>
            </Card>
          </Col>
        ))}
      </Row>
    </div>
  );
};

// 会话记录组件
const SessionsPage = () => {
  const [sessions] = useState([
    {
      id: 'sess001',
      device: '客厅音箱',
      user: '用户',
      time: '2024-10-24 16:25:00',
      input: '今天天气怎么样',
      output: '今天天气晴朗，温度25度，适合外出活动',
      duration: '3.2秒',
      status: 'completed'
    },
    {
      id: 'sess002',
      device: '厨房中控',
      user: '用户',
      time: '2024-10-24 16:20:00',
      input: '播放音乐',
      output: '正在为您播放音乐',
      duration: '2.1秒',
      status: 'completed'
    },
    {
      id: 'sess003',
      device: '卧室显示屏',
      user: '用户',
      time: '2024-10-24 16:15:00',
      input: '打开卧室灯',
      output: '已为您打开卧室的灯',
      duration: '1.8秒',
      status: 'completed'
    },
    {
      id: 'sess004',
      device: '客厅音箱',
      user: '用户',
      time: '2024-10-24 16:10:00',
      input: '设置提醒',
      output: '请告诉我您想设置什么提醒',
      duration: '进行中',
      status: 'active'
    }
  ]);

  const getStatusColor = (status: string) => {
    return status === 'completed' ? 'success' : 'processing';
  };

  const getStatusText = (status: string) => {
    return status === 'completed' ? '已完成' : '进行中';
  };

  return (
    <div style={{ padding: 24 }}>
      <div style={{ marginBottom: 24 }}>
        <h2>会话记录</h2>
        <p style={{ color: '#8c8c8c' }}>查看历史语音交互记录</p>
      </div>

      <Card>
        <List
          dataSource={sessions}
          renderItem={(session) => (
            <List.Item>
              <List.Item.Meta
                avatar={<Avatar icon={<AudioOutlined />} />}
                title={
                  <div>
                    <div style={{ fontWeight: 500, marginBottom: 4 }}>
                      {session.device} - {session.user}
                    </div>
                    <div style={{ fontSize: 12, color: '#8c8c8c' }}>
                      {session.time}
                    </div>
                  </div>
                }
                description={
                  <div>
                    <div style={{ marginBottom: 8 }}>
                      <strong>用户:</strong> "{session.input}"
                    </div>
                    <div>
                      <strong>助手:</strong> "{session.output}"
                    </div>
                    <div style={{ marginTop: 8, fontSize: 12, color: '#8c8c8c' }}>
                      <span>时长: {session.duration}</span>
                      <Tag color={getStatusColor(session.status)} style={{ marginLeft: 8 }}>
                        {getStatusText(session.status)}
                      </Tag>
                    </div>
                  </div>
                }
              />
            </List.Item>
          )}
        />
      </Card>
    </div>
  );
};

// 系统设置组件
const SettingsPage = () => {
  const [settings, setSettings] = useState({
    autoBackup: true,
    logLevel: 'info',
    sessionTimeout: 30,
    maxDevices: 100,
    notifications: true
  });

  return (
    <div style={{ padding: 24 }}>
      <div style={{ marginBottom: 24 }}>
        <h2>系统设置</h2>
        <p style={{ color: '#8c8c8c' }}>配置系统参数和偏好设置</p>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card title="基本设置">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>自动备份</span>
                <Switch
                  checked={settings.autoBackup}
                  onChange={(checked) => setSettings({...settings, autoBackup: checked})}
                />
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>通知提醒</span>
                <Switch
                  checked={settings.notifications}
                  onChange={(checked) => setSettings({...settings, notifications: checked})}
                />
              </div>
              <div>
                <div style={{ marginBottom: 8 }}>日志级别</div>
                <Select
                  value={settings.logLevel}
                  onChange={(value) => setSettings({...settings, logLevel: value})}
                  style={{ width: '100%' }}
                  options={[
                    { value: 'debug', label: 'Debug' },
                    { value: 'info', label: 'Info' },
                    { value: 'warning', label: 'Warning' },
                    { value: 'error', label: 'Error' }
                  ]}
                />
              </div>
            </div>
          </Card>
        </Col>

        <Col xs={24} lg={12}>
          <Card title="高级设置">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              <div>
                <div style={{ marginBottom: 8 }}>会话超时时间（分钟）</div>
                <InputNumber
                  value={settings.sessionTimeout}
                  onChange={(value) => setSettings({...settings, sessionTimeout: value || 30})}
                  min={1}
                  max={120}
                  style={{ width: '100%' }}
                />
              </div>
              <div>
                <div style={{ marginBottom: 8 }}>最大设备数量</div>
                <InputNumber
                  value={settings.maxDevices}
                  onChange={(value) => setSettings({...settings, maxDevices: value || 100})}
                  min={1}
                  max={1000}
                  style={{ width: '100%' }}
                />
              </div>
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
        <Col xs={24}>
          <Card title="系统信息">
            <Row gutter={[16, 16]}>
              <Col xs={24} sm={8}>
                <Statistic title="系统版本" value="1.0.0" />
              </Col>
              <Col xs={24} sm={8}>
                <Statistic title="运行时间" value="2小时15分钟" />
              </Col>
              <Col xs={24} sm={8}>
                <Statistic title="数据库状态" value="正常" valueStyle={{ color: '#3f8600' }} />
              </Col>
            </Row>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

// 主应用组件
function App() {
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const [selectedMenu, setSelectedMenu] = useState('dashboard');

  // 菜单配置
  const menuItems = [
    {
      key: 'dashboard',
      icon: <DashboardOutlined />,
      label: '仪表板'
    },
    {
      key: 'devices',
      icon: <AudioOutlined />,
      label: '设备管理'
    },
    {
      key: 'sessions',
      icon: <HistoryOutlined />,
      label: '会话记录'
    },
    {
      key: 'settings',
      icon: <SettingOutlined />,
      label: '系统设置'
    }
  ];

  const handleMenuClick = ({ key }: { key: string }) => {
    setSelectedMenu(key);
  };

  // 如果未登录，显示登录页面
  if (!isLoggedIn) {
    return (
      <ConfigProvider locale={zhCN}>
        <LoginPage onLogin={() => setIsLoggedIn(true)} />
      </ConfigProvider>
    );
  }

  // 已登录，显示管理界面
  const renderContent = () => {
    switch (selectedMenu) {
      case 'dashboard':
        return <DashboardPage />;
      case 'devices':
        return <DevicesPage />;
      case 'sessions':
        return <SessionsPage />;
      case 'settings':
        return <SettingsPage />;
      default:
        return <DashboardPage />;
    }
  };

  return (
    <ConfigProvider locale={zhCN}>
      <Layout style={{ minHeight: '100vh' }}>
        {/* 侧边栏 */}
        <Sider
          trigger={null}
          collapsible
          collapsed={collapsed}
          style={{
            background: '#fff',
            boxShadow: '2px 0 8px rgba(0,0,0,0.15)'
          }}
        >
          {/* Logo */}
          <div
            style={{
              height: 64,
              display: 'flex',
              alignItems: 'center',
              justifyContent: collapsed ? 'center' : 'flex-start',
              padding: collapsed ? 0 : '0 24px',
              borderBottom: '1px solid #f0f0f0'
            }}
          >
            {collapsed ? (
              <div
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: '50%',
                  background: '#1890ff',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'white',
                  fontWeight: 'bold'
                }}
              >
                E
              </div>
            ) : (
              <div>
                <div style={{ fontSize: 18, fontWeight: 'bold', color: '#1890ff' }}>
                  Echo Web
                </div>
                <div style={{ fontSize: 12, color: '#8c8c8c' }}>
                  智能音箱管理平台
                </div>
              </div>
            )}
          </div>

          {/* 菜单 */}
          <Menu
            mode="inline"
            selectedKeys={[selectedMenu]}
            items={menuItems}
            onClick={handleMenuClick}
            style={{ border: 'none' }}
          />
        </Sider>

        <Layout>
          {/* 顶部导航 */}
          <Header
            style={{
              padding: '0 16px',
              background: '#fff',
              borderBottom: '1px solid #f0f0f0',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between'
            }}
          >
            {/* 左侧：折叠按钮 */}
            <div style={{ display: 'flex', alignItems: 'center' }}>
              <Button
                type="text"
                icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                onClick={() => setCollapsed(!collapsed)}
                style={{
                  fontSize: '16px',
                  width: 64,
                  height: 64
                }}
              />
            </div>

            {/* 右侧：用户信息 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <span style={{ fontSize: 14, color: '#1890ff' }}>
                在线设备: 2
              </span>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  cursor: 'pointer',
                  padding: '4px 8px',
                  borderRadius: 6
                }}
              >
                <Avatar size="small" icon={<UserOutlined />} />
                <span style={{ fontSize: 14 }}>管理员</span>
                <Button
                  type="link"
                  size="small"
                  onClick={() => setIsLoggedIn(false)}
                >
                  退出
                </Button>
              </div>
            </div>
          </Header>

          {/* 主内容区域 */}
          <Content
            style={{
              margin: 0,
              background: '#f5f5f5',
              minHeight: 'calc(100vh - 64px)',
              overflow: 'auto'
            }}
          >
            {renderContent()}
          </Content>
        </Layout>
      </Layout>
    </ConfigProvider>
  );
}

export default App;
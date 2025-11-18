import { ConfigProvider, Button, Card, Input, Form, Layout, Menu, Avatar, Row, Col, Statistic, Table, Tag, List, Timeline, Switch, InputNumber, Select } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { UserOutlined, LockOutlined, DashboardOutlined, AudioOutlined, HistoryOutlined, SettingOutlined, MenuFoldOutlined, MenuUnfoldOutlined, PlayCircleOutlined, WifiOutlined, ReloadOutlined } from '@ant-design/icons';
import { useState } from 'react';
import { BrowserRouter } from 'react-router-dom';
import DeviceList from './pages/DeviceList';
import { Dashboard } from './pages/Dashboard';
import { Sessions } from './pages/Sessions';
import { Settings } from './pages/Settings';
import useDeviceStore from './stores/useDeviceStore';
import useSessionStore from './stores/useSessionStore';
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


// 主应用组件
function App() {
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const [selectedMenu, setSelectedMenu] = useState('dashboard');

  // 获取设备store和会话store的数据
  const { stats: deviceStats } = useDeviceStore();
  const { stats: sessionStats } = useSessionStore();

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
      <BrowserRouter>
        <ConfigProvider locale={zhCN}>
          <LoginPage onLogin={() => setIsLoggedIn(true)} />
        </ConfigProvider>
      </BrowserRouter>
    );
  }

  // 已登录，显示管理界面
  const renderContent = () => {
    switch (selectedMenu) {
      case 'dashboard':
        return <Dashboard />;
      case 'devices':
        return <DeviceList />;
      case 'sessions':
        return <Sessions />;
      case 'settings':
        return <Settings />;
      default:
        return <Dashboard />;
    }
  };

  return (
    <BrowserRouter>
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
                  在线设备: {deviceStats.online || 0}
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
    </BrowserRouter>
  );
}

export default App;
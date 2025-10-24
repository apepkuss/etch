import React, { useState } from 'react';
import { Layout as AntLayout, Menu, Avatar, Button } from 'antd';
import {
  DashboardOutlined,
  AudioOutlined,
  HistoryOutlined,
  SettingOutlined,
  UserOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined
} from '@ant-design/icons';
import { useNavigate, useLocation, Outlet } from 'react-router-dom';
import { useDeviceStore } from '../stores/useDeviceStore';

const { Header, Sider, Content } = AntLayout;

export const Layout: React.FC = () => {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const { stats } = useDeviceStore();

  // 菜单配置
  const menuItems = [
    {
      key: '/dashboard',
      icon: <DashboardOutlined />,
      label: '仪表板'
    },
    {
      key: '/devices',
      icon: <AudioOutlined />,
      label: '设备管理'
    },
    {
      key: '/sessions',
      icon: <HistoryOutlined />,
      label: '会话记录'
    },
    {
      key: '/settings',
      icon: <SettingOutlined />,
      label: '系统设置'
    }
  ];

  const handleMenuClick = ({ key }: { key: string }) => {
    navigate(key);
  };

  return (
    <AntLayout style={{ minHeight: '100vh' }}>
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
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={handleMenuClick}
          style={{ border: 'none' }}
        />
      </Sider>

      <AntLayout>
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

          {/* 右侧：用户信息和统计 */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
            {/* 连接状态指示器 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  backgroundColor: '#52c41a'
                }}
              />
              <span style={{ fontSize: 12, color: '#8c8c8c' }}>
                已连接
              </span>
            </div>

            {/* 设备统计 */}
            <span style={{ fontSize: 14, color: '#1890ff' }}>
              在线设备: {stats.online}
            </span>

            {/* 用户信息 */}
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
          <Outlet />
        </Content>
      </AntLayout>
    </AntLayout>
  );
};
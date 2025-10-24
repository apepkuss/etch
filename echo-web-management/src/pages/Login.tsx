import React, { useState } from 'react';
import { Card, Form, Input, Button, message, Space, Divider } from 'antd';
import {
  UserOutlined,
  LockOutlined,
  AudioOutlined,
  EyeInvisibleOutlined,
  EyeTwoTone
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';

export const Login: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleLogin = async (values: any) => {
    setLoading(true);
    try {
      // 模拟登录API调用
      await new Promise(resolve => setTimeout(resolve, 1000));

      // 简单的验证逻辑（实际应该调用后端API）
      if (values.username === 'admin' && values.password === 'admin123') {
        message.success('登录成功');
        localStorage.setItem('token', 'mock-token');
        navigate('/dashboard');
      } else {
        message.error('用户名或密码错误');
      }
    } catch (error) {
      message.error('登录失败，请稍后重试');
    } finally {
      setLoading(false);
    }
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
            <AudioOutlined />
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
              iconRender={(visible) =>
                visible ? <EyeTwoTone /> : <EyeInvisibleOutlined />
              }
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

        <Divider>
          <span style={{ color: '#8c8c8c', fontSize: 12 }}>演示账户</span>
        </Divider>

        {/* 演示账户信息 */}
        <div
          style={{
            background: '#f6f8fa',
            padding: 16,
            borderRadius: 6,
            fontSize: 12
          }}
        >
          <div style={{ marginBottom: 8, fontWeight: 500 }}>测试账户：</div>
          <div>用户名：admin</div>
          <div>密码：admin123</div>
        </div>

        {/* 版本信息 */}
        <div style={{ textAlign: 'center', marginTop: 24 }}>
          <span style={{ color: '#8c8c8c', fontSize: 12 }}>
            Version 1.0.0
          </span>
        </div>
      </Card>
    </div>
  );
};
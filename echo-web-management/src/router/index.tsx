import React from 'react';
import { createBrowserRouter, RouterProvider, Navigate } from 'react-router-dom';
import { Layout } from '../components/Layout';
import { Dashboard } from '../pages/Dashboard';
import { DeviceList } from '../pages/DeviceList';
import { DeviceDetail } from '../pages/DeviceDetail';
import { Sessions } from '../pages/Sessions';
import { Settings } from '../pages/Settings';
import { Login } from '../pages/Login';

// 路由配置
const router = createBrowserRouter([
  {
    path: '/login',
    element: <Login />
  },
  {
    path: '/',
    element: <Layout />,
    children: [
      {
        index: true,
        element: <Navigate to="/dashboard" replace />
      },
      {
        path: 'dashboard',
        element: <Dashboard />
      },
      {
        path: 'devices',
        children: [
          {
            index: true,
            element: <DeviceList />
          },
          {
            path: ':deviceId',
            element: <DeviceDetail />
          }
        ]
      },
      {
        path: 'sessions',
        element: <Sessions />
      },
      {
        path: 'settings',
        element: <Settings />
      }
    ]
  }
]);

export const AppRouter: React.FC = () => {
  return <RouterProvider router={router} />;
};
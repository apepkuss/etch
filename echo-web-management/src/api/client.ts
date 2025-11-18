import axios, { AxiosInstance, AxiosResponse } from 'axios';
import { ApiResponse } from '../types';

// API 配置
const API_BASE_URL = 'http://localhost:8080';
const API_PREFIX = '/api/v1';

// 创建 axios 实例
const apiClient: AxiosInstance = axios.create({
  baseURL: `${API_BASE_URL}${API_PREFIX}`,
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
});

// 请求拦截器
apiClient.interceptors.request.use(
  (config) => {
    // 这里可以添加认证 token
    let token = localStorage.getItem('authToken');

    // 临时解决方案：如果没有token，自动设置一个admin token用于演示
    if (!token) {
      token = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJhZG1pbi0wMDEiLCJ1c2VybmFtZSI6ImFkbWluIiwicm9sZSI6IkFkbWluIiwiZXhwIjoxNzYzNDc1NjM3LCJpYXQiOjE3NjMzODkyMzd9.FoofLi2-w3EqVrHgUTYy8ne14dpsYfeWOXyNdj3Oia8';
      localStorage.setItem('authToken', token);
    }

    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error) => {
    return Promise.reject(error);
  }
);

// 响应拦截器
apiClient.interceptors.response.use(
  (response: AxiosResponse<ApiResponse<any>>) => {
    return response;
  },
  (error) => {
    console.error('API Error:', error);

    // 处理不同的错误状态码
    if (error.response) {
      switch (error.response.status) {
        case 401:
          // 未授权，清除 token 并跳转到登录页
          localStorage.removeItem('authToken');
          window.location.href = '/login';
          break;
        case 403:
          console.error('Access forbidden');
          break;
        case 404:
          console.error('Resource not found');
          break;
        case 409:
          console.error('Conflict - Resource already exists');
          break;
        case 500:
          console.error('Server error');
          break;
        default:
          console.error('Unknown error');
      }
    } else if (error.request) {
      console.error('Network error');
    } else {
      console.error('Request error');
    }

    return Promise.reject(error);
  }
);

export default apiClient;
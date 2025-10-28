#!/usr/bin/env python3
"""
简单的存储层测试脚本
验证PostgreSQL和Redis连接是否正常
"""

import psycopg2
import redis
import json
from datetime import datetime

def test_postgresql():
    """测试PostgreSQL连接"""
    print("🐘 测试PostgreSQL连接...")
    try:
        # 连接到PostgreSQL
        conn = psycopg2.connect(
            host="localhost",
            port=5432,
            database="echo_db",
            user="echo_user",
            password="echo_password"
        )
        cur = conn.cursor()

        # 测试查询
        cur.execute("SELECT version();")
        version = cur.fetchone()
        print(f"  ✅ PostgreSQL连接成功: {version[0]}")

        # 测试用户表
        cur.execute("SELECT COUNT(*) FROM users;")
        user_count = cur.fetchone()[0]
        print(f"  👥 用户表记录数: {user_count}")

        # 测试设备表
        cur.execute("SELECT COUNT(*) FROM devices;")
        device_count = cur.fetchone()[0]
        print(f"  🔊 设备表记录数: {device_count}")

        # 插入测试用户
        cur.execute("""
            INSERT INTO users (username, email, password_hash, role)
            VALUES (%s, %s, %s, %s)
            ON CONFLICT (username) DO NOTHING
            RETURNING id;
        """, ("test_user_python", "test@python.local", "hashed_password", "user"))

        result = cur.fetchone()
        if result:
            print(f"  🆕 创建测试用户ID: {result[0]}")

        conn.commit()
        cur.close()
        conn.close()

        return True
    except Exception as e:
        print(f"  ❌ PostgreSQL连接失败: {e}")
        return False

def test_redis():
    """测试Redis连接"""
    print("\n💾 测试Redis连接...")
    try:
        # 连接到Redis
        r = redis.Redis(
            host="localhost",
            port=6379,
            password="redis_password",
            decode_responses=True
        )

        # 测试连接
        r.ping()
        print("  ✅ Redis连接成功")

        # 测试基本操作
        test_key = "test:storage:python"
        test_value = {
            "timestamp": datetime.now().isoformat(),
            "message": "Hello from Python storage test!"
        }

        # 设置值
        r.set(test_key, json.dumps(test_value), ex=60)
        print("  📝 Redis写入测试成功")

        # 获取值
        retrieved = r.get(test_key)
        if retrieved:
            data = json.loads(retrieved)
            print(f"  📖 Redis读取测试成功: {data['message']}")

        # 清理测试数据
        r.delete(test_key)

        return True
    except Exception as e:
        print(f"  ❌ Redis连接失败: {e}")
        return False

def test_data_integration():
    """测试数据集成"""
    print("\n🔄 测试数据集成...")
    try:
        # 连接PostgreSQL
        pg_conn = psycopg2.connect(
            host="localhost",
            port=5432,
            database="echo_db",
            user="echo_user",
            password="echo_password"
        )
        pg_cur = pg_conn.cursor()

        # 连接Redis
        r = redis.Redis(
            host="localhost",
            port=6379,
            password="redis_password",
            decode_responses=True
        )

        # 获取用户列表
        pg_cur.execute("""
            SELECT id, username, email, role, created_at
            FROM users
            ORDER BY created_at DESC
            LIMIT 5;
        """)
        users = pg_cur.fetchall()

        print(f"  👥 从PostgreSQL获取 {len(users)} 个用户:")

        # 将用户列表缓存到Redis
        cache_key = "users:recent"
        users_data = [
            {
                "id": str(user[0]),
                "username": user[1],
                "email": user[2],
                "role": user[3],
                "created_at": user[4].isoformat() if user[4] else None
            }
            for user in users
        ]

        r.set(cache_key, json.dumps(users_data), ex=300)  # 5分钟过期
        print(f"  💾 用户列表已缓存到Redis (key: {cache_key})")

        # 从Redis验证缓存
        cached_users = json.loads(r.get(cache_key))
        print(f"  📖 从Redis读取缓存成功，共 {len(cached_users)} 个用户")

        pg_cur.close()
        pg_conn.close()

        return True
    except Exception as e:
        print(f"  ❌ 数据集成测试失败: {e}")
        return False

def main():
    """主函数"""
    print("🚀 开始Echo系统存储层测试...\n")

    # 测试PostgreSQL
    pg_ok = test_postgresql()

    # 测试Redis
    redis_ok = test_redis()

    # 测试数据集成
    integration_ok = test_data_integration()

    # 总结
    print("\n" + "="*50)
    print("📊 测试结果总结:")
    print(f"  PostgreSQL: {'✅ 正常' if pg_ok else '❌ 异常'}")
    print(f"  Redis: {'✅ 正常' if redis_ok else '❌ 异常'}")
    print(f"  数据集成: {'✅ 正常' if integration_ok else '❌ 异常'}")

    if pg_ok and redis_ok and integration_ok:
        print("\n🎉 存储层测试全部通过！系统可以正常工作。")
        return True
    else:
        print("\n⚠️  部分测试失败，请检查相关服务。")
        return False

if __name__ == "__main__":
    main()
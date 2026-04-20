#ifndef __CORE_THREAD_H__
#define __CORE_THREAD_H__

#include <thread>
#include <string>
#include <functional>
#include <atomic>
#include <chrono>

#ifdef _WIN32
#include <windows.h>
#include <processthreadsapi.h>
#else
#include <pthread.h>
#endif

class CoreThread
{
public:
    /// @brief 新建一个线程
    /// @param name 线程名字
    /// @param task 线程内容
    CoreThread(const std::string &name, std::function<void()> task)
        : name_(name), task_(task), running_(false), joined_(false)
    {
    }

    // 禁止拷贝
    CoreThread(const CoreThread &) = delete;
    CoreThread &operator=(const CoreThread &) = delete;

    // 允许移动
    CoreThread(CoreThread &&other) noexcept
        : name_(std::move(other.name_)), task_(std::move(other.task_)), thread_(std::move(other.thread_)), running_(other.running_.load()), joined_(other.joined_)
    {
        other.joined_ = true;
    }

    /// @brief 线程结束
    ~CoreThread()
    {
        if (running_ && !joined_)
        {
            Stop();
        }
    }

    /// @brief 启动线程
    /// @return 是否启动成功
    bool Start();

    /// @brief 停止线程 会阻塞
    void Stop();

    /// @brief 分离线程
    void Detach();

    /// @brief 等待线程结束
    void Join();

    /// @brief 线程是否在运行
    /// @return 运行状态
    bool IsRunning() const
    {
        return running_;
    }

    /// @brief 获取线程名字
    /// @return 线程名字
    std::string GetName() const
    {
        return name_;
    }

    /// @brief 获取线程ID
    /// @return 线程ID
    std::thread::id GetId() const
    {
        return thread_.get_id();
    }

private:
    /// @brief 线程运行
    void Run();

    /// @brief 设置线程名称
    /// @param name 名字
    static void SetThreadName(const std::string &name);

    /// @brief 线程名字
    std::string name_;
    /// @brief 线程内容
    std::function<void()> task_;
    /// @brief 线程
    std::thread thread_;
    /// @brief 是否在运行
    std::atomic<bool> running_;
    /// @brief 是否等待停止
    bool joined_;
};

#endif // __CORE_THREAD_H__
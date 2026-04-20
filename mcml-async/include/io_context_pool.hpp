#ifndef __UI_CONTEXT_POOL_H__
#define __UI_CONTEXT_POOL_H__

#include <atomic>
#include <boost/asio.hpp>
#include <functional>
#include <iostream>
#include <memory>
#include <thread>
#include <vector>
#include "core_thread.hpp"

class IoContextPool
{
public:
    /// @brief 获取线程池
    /// @return 线程池
    static IoContextPool &Instance() { return instance_; }

    // 禁止拷贝和移动
    IoContextPool(const IoContextPool &) = delete;
    IoContextPool &operator=(const IoContextPool &) = delete;

    /// @brief 获取 io_context（用于创建 socket、resolver 等）
    /// @return io_context
    boost::asio::io_context &GetContext() { return io_context_; }

    /// @brief 获取执行器（用于 post、dispatch 等）
    /// @return 执行器
    auto GetExecutor() { return io_context_.get_executor(); }

    /// @brief 启动线程池
    /// @param thread_count 指定线程数，0 表示自动检测
    void Start(size_t thread_count = 0);

    /// @brief 停止
    void Stop();

    /// @brief 重启
    void Restart();

    /// @brief 检查是否运行中
    /// @return 运行状态
    bool IsRunning() const { return running_ && started_; }

    template <typename F>
    void Post(F &&handler)
    {
        boost::asio::post(io_context_, std::forward<F>(handler));
    }

    /// @brief 获取线程数量
    /// @return 线程数量
    size_t GetThreadCount() const { return workers_.size(); }

    ~IoContextPool() { Stop(); }

private:
    static IoContextPool instance_;

    IoContextPool() : io_context_(), running_(true), started_(false) {}

    void Run(int i);

    /// @brief io_context
    boost::asio::io_context io_context_;
    /// @brief 工作线程列表
    std::vector<CoreThread> workers_;
    /// @brief 是否在运行
    std::atomic<bool> running_;
    /// @brief 是否启动了
    bool started_;
};

#endif // __UI_CONTEXT_POOL_H__
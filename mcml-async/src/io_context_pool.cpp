#include "io_context_pool.hpp"
#include "log.hpp"

void IoContextPool::Start(size_t thread_count)
{
    if (started_)
        return;

    if (thread_count == 0)
    {
        thread_count = std::thread::hardware_concurrency();
    }

    Log::Info(std::format("MCML 工作线程池启动，线程数量 {}", thread_count));

    started_ = true;
    for (size_t i = 0; i < thread_count; ++i)
    {
        workers_.push_back(CoreThread(std::format("MCML Worker {}", i), [this, idx = i]() { Run(idx); }));
    }
}

void IoContextPool::Run(int i)
{
    Log::Info(std::format("MCML 工作线程 {} 启动", i));

    while (running_) 
    {
        try 
        {
            io_context_.run();
            if (!running_) break;
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        catch (const std::exception& e) 
        {
            Log::Error(std::format("MCML 工作线程 {} 出现错误，{}", i, e.what()));
        }
    }
}

void IoContextPool::Stop()
{
    if (!started_)
        return;

    Log::Info("MCML 工作线程池停止");
    running_ = false;

    io_context_.stop();

    for (auto &worker : workers_)
    {
        worker.Join();
    }
    workers_.clear();

    io_context_.restart();
    started_ = false;
}

void IoContextPool::Restart()
{
    Log::Info("MCML 工作线程池重启");

    if (started_)
    {
        Stop();
    }
    io_context_.restart();
    running_ = true;
    started_ = false;
}
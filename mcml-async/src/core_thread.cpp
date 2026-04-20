#include "core_thread.hpp"

bool CoreThread::Start()
{
    if (running_)
    {
        return false; // 已经在运行
    }

    running_ = true;
    thread_ = std::thread([this]() { Run(); });
    return true;
}

void CoreThread::Stop()
{
    if (!running_)
    {
        return;
    }

    running_ = false;
    if (thread_.joinable())
    {
        thread_.join();
        joined_ = true;
    }
}

void CoreThread::Detach()
{
    if (thread_.joinable())
    {
        thread_.detach();
        joined_ = true;
    }
}

void CoreThread::Join()
{
    if (thread_.joinable())
    {
        thread_.join();
        joined_ = true;
    }
}

void CoreThread::Run()
{
    SetThreadName(name_);

    // 执行任务
    if (task_)
    {
        task_();
    }

    running_ = false;
}

void CoreThread::SetThreadName(const std::string &name)
{
#ifdef _WIN32
    // Windows 10+ 方式
    std::wstring wname(name.begin(), name.end());
    SetThreadDescription(GetCurrentThread(), wname.c_str());
#elif __linux__
    // Linux：最多15个字符
    pthread_setname_np(pthread_self(), name.substr(0, 15).c_str());
#elif __APPLE__
    // macOS
    pthread_setname_np(name.c_str());
#endif
}
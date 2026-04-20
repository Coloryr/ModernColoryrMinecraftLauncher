#ifndef __LOG_H__
#define __LOG_H__

#include <string>
#include <fstream>
#include <iostream>
#include <sstream>
#include <thread>
#include <filesystem>
#include <semaphore>
#include <boost/lockfree/queue.hpp>
#include "core_thread.hpp"

/// @brief 日志等级
enum LogLevel
{
    LOG_INFO = 1 << 0,
    LOG_WARN = 1 << 1,
    LOG_ERROR = 1 << 2,
    LOG_FATAL = 1 << 3
};

class Log
{
private:
    struct LogItem
    {
        std::string* log;
        LogLevel level;
    };

    inline static std::filesystem::path log_file_;
    inline static std::fstream log_stream_;

    inline static bool init_;
    inline static CoreThread* log_thread_;
    inline static boost::lockfree::queue<LogItem*> log_queue_{ 128 };
    inline static std::counting_semaphore<1024> log_semaphore_{ 0 };

    inline static int log_level_;

    inline static bool run_;

    static void PutLog(LogLevel level, std::string* log);

    static const char* GetLogLevelName(LogLevel level)
    {
        switch (level) 
        {
            case LOG_INFO:  return "INFO";
            case LOG_WARN:  return "WARN";
            case LOG_ERROR: return "ERROR";
            case LOG_FATAL: return "FATAL";
            default:        return "UNKNOWN";
        }
    }

public:
    static void Init(const std::string& local);
    static void Stop();

    static void Info(const std::string& str);
    static void Warn(const std::string& str);
    static void Error(const std::string& str);
    static void Fatal(const std::string& str);

    static void Run();
};

#endif // __LOG_H__
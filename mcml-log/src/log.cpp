#include "log.hpp"

void Log::Init(const std::string &local)
{
    log_level_ = LogLevel::LOG_INFO | LogLevel::LOG_WARN | LogLevel::LOG_ERROR | LogLevel::LOG_FATAL;

    log_file_ = std::filesystem::path(local) / "log.logs";
    log_stream_.open(log_file_, std::ios::app);
    
    if (log_stream_.is_open())
    {
        init_ = true;
        run_ = true;
        log_thread_ = new CoreThread("mcml log", Run);
    }
    else
    {
        init_ = false;
    }
}

void Log::Stop()
{
    if (run_)
    {
        log_semaphore_.release();
    }
    run_ = false;
}

void Log::PutLog(LogLevel level, std::string* log)
{
    if (log_level_ | level)
    {
        LogItem* item = new LogItem();
        item->level = level;
        item->log = log;

        log_queue_.push(item);
        log_semaphore_.release();
    }
}

void Log::Info(const std::string &str)
{
    PutLog(LogLevel::LOG_INFO, new std::string(str));
}

void Log::Warn(const std::string &str)
{
    PutLog(LogLevel::LOG_WARN, new std::string(str));
}

void Log::Error(const std::string &str) 
{
    PutLog(LogLevel::LOG_ERROR, new std::string(str));
}

void Log::Fatal(const std::string &str) 
{
    PutLog(LogLevel::LOG_FATAL, new std::string(str));
}

void Log::Run()
{
    while (run_)
    {
        log_semaphore_.acquire();
        LogItem* item;
        while (log_queue_.pop(item))
        {
            time_t nowtime;
            time(&nowtime);
            tm* p = localtime(&nowtime);
            std::string time = std::format("{}/{}/{} {}-{}-{}", p->tm_year, p->tm_mon + 1, p->tm_mday,
                p->tm_hour, p->tm_min, p->tm_sec);
            log_stream_ << '[' << GetLogLevelName(item->level) << ']' << '[' << time << ']' << item->log->c_str() << "\r\n";
        }
    }
}
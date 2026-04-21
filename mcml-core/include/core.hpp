#ifndef __CORE_H__
#define __CORE_H__

#include <boost/asio.hpp>
#include <memory>
#include <thread>
#include <vector>
#include <atomic>
#include <condition_variable>
#include "event.hpp"

struct CoreInitArg
{
    std::string local;
    std::string oauth_key;
    std::string curseforge_key;
};

class Core
{
private:
    inline static CoreInitArg* arg;
    inline static Event<void> core_stop_event;

public:
    static void Start(CoreInitArg* arg);
    static void Stop();

    static CoreInitArg* GetArg() { return arg; }
    static void AddStopEvent(std::function<void()> handel)
    {
        core_stop_event.AddEventHandel(handel);
    }
};


#endif // __CORE_H__
#ifndef __CORE_H__
#define __CORE_H__

#include <boost/asio.hpp>
#include <memory>
#include <thread>
#include <vector>
#include <atomic>
#include <condition_variable>

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

public:
    static void Start(CoreInitArg* arg);
    static void Stop();

    static CoreInitArg* GetArg() { return arg; }
};


#endif // __CORE_H__
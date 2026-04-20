#include "core.hpp"
#include "log.hpp"

void Core::Start(CoreInitArg* arg)
{
    Core::arg = arg;

    Log::Init(arg->local);
}

void Core::Stop()
{
    
}
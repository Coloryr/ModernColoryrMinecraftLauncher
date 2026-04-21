#include "core.hpp"
#include "log.hpp"

void Core::Start(CoreInitArg* arg)
{
    Core::arg = arg;

    Log::Init(arg->local);



    AddStopEvent([] { Log::Stop(); });
}

void Core::Stop()
{
    core_stop_event.SendEvent();
}
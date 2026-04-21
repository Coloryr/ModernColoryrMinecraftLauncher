#ifndef __EVENT_H__
#define __EVENT_H__

#include <list>
#include <functional>

template<typename T>
class Event
{
public:
	void AddEventHandel(std::function<void(T)> handel)
	{
		list_.push_back(handel);
	}

	void SendEvent(T data)
	{
		for(std::function<void(T)> var : list_)
		{
			var(data);
		}
	}
private:
	std::list<std::function<void(T)>> list_;
};

template<>
class Event<void>
{
public:
	void AddEventHandel(std::function<void()> handel)
	{
		list_.push_back(handel);
	}

	void SendEvent()
	{
		for (std::function<void()> var : list_)
		{
			var();
		}
	}

private:
	std::list<std::function<void()>> list_;
};

#endif // __EVENT_H__
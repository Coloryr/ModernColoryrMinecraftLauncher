#ifndef __HTTP_CLIENT_H__
#define __HTTP_CLIENT_H__

#include "io_context_pool.hpp"
#include <boost/asio/awaitable.hpp>
#include <boost/asio/use_awaitable.hpp>
#include <boost/asio/co_spawn.hpp>
#include <boost/asio/detached.hpp>
#include <boost/beast/core.hpp>
#include <boost/beast/http.hpp>
#include <chrono>
#include <string>
#include <functional>
#include <memory>

namespace net = boost::asio;
namespace beast = boost::beast;
namespace http = beast::http;

class HttpClient
{
public:
    net::awaitable<std::string> get_string_async(const std::string &requestUri);

    net::awaitable<http::response<http::string_body>> get_async(const std::string &requestUri);

private:
    struct UrlParts
    {
        std::string host;
        std::string port;
        std::string target;
        bool is_https;
    };

    static net::io_context &get_io_context() { return IoContextPool::Instance().GetContext(); }

    UrlParts parse_url(const std::string &url);

    net::awaitable<http::response<http::string_body>> send_async(const UrlParts &parts);
    net::awaitable<http::response<http::string_body>> send_async(const UrlParts &parts,
                                                                 std::chrono::seconds timeout);

    std::chrono::seconds default_timeout_{10};
    std::string user_agent_{"MCML/1.0"};
};

#endif // __HTTP_CLIENT_H__
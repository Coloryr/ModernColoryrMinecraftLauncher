#include "net/http_client.hpp"
#include "net/ssl_context.hpp"
#include <regex>

namespace beast = boost::beast;
namespace http = beast::http;

using tcp = net::ip::tcp;

HttpClient::UrlParts HttpClient::parse_url(const std::string &url)
{
    std::regex re(R"(^(https?)://([^/:]+)(?::(\d+))?(/.*)?$)");
    std::smatch m;

    if (!std::regex_match(url, m, re))
    {
        throw std::runtime_error("Invalid URL: " + url);
    }

    std::string protocol = m[1];
    bool is_https = (protocol == "https");

    return {
        m[2],                                                  // host
        m[3].matched ? m[3].str() : (is_https ? "443" : "80"), // port
        m[4].matched ? m[4].str() : "/",                       // target
        is_https};
}

net::awaitable<http::response<http::string_body>>
HttpClient::send_async(const UrlParts &parts)
{
    return send_async(parts, default_timeout_);
}

net::awaitable<http::response<http::string_body>>
HttpClient::send_async(const UrlParts &parts, std::chrono::seconds timeout)
{
    auto &ctx = get_io_context();

    if (parts.is_https)
    {
        // HTTPS 请求
        tcp::resolver resolver(ctx);
        beast::ssl_stream<beast::tcp_stream> stream(ctx, SslContext::Instance().get_context());

        // 设置超时
        beast::get_lowest_layer(stream).expires_after(timeout);

        // 解析域名
        auto results = co_await resolver.async_resolve(parts.host, parts.port, net::use_awaitable);

        // 连接
        co_await beast::get_lowest_layer(stream).async_connect(results, net::use_awaitable);

        // SSL 握手
        beast::get_lowest_layer(stream).expires_after(timeout);
        co_await stream.async_handshake(ssl::stream_base::client, net::use_awaitable);

        // 构造请求
        http::request<http::string_body> req{http::verb::get, parts.target, 11};
        req.set(http::field::host, parts.host);
        req.set(http::field::user_agent, user_agent_);

        // 发送请求
        beast::get_lowest_layer(stream).expires_after(timeout);
        co_await http::async_write(stream, req, net::use_awaitable);

        // 接收响应
        http::response<http::string_body> res;
        beast::flat_buffer buffer;
        beast::get_lowest_layer(stream).expires_after(timeout);
        co_await http::async_read(stream, buffer, res, net::use_awaitable);

        // SSL 关闭
        stream.async_shutdown();

        co_return res;
    }
    else
    {
        // HTTP 请求
        tcp::resolver resolver(ctx);
        beast::tcp_stream stream(ctx);

        stream.expires_after(timeout);

        auto results = co_await resolver.async_resolve(parts.host, parts.port, net::use_awaitable);
        co_await stream.async_connect(results, net::use_awaitable);

        http::request<http::string_body> req{http::verb::get, parts.target, 11};
        req.set(http::field::host, parts.host);
        req.set(http::field::user_agent, user_agent_);

        stream.expires_after(timeout);
        co_await http::async_write(stream, req, net::use_awaitable);

        http::response<http::string_body> res;
        beast::flat_buffer buffer;
        stream.expires_after(timeout);
        co_await http::async_read(stream, buffer, res, net::use_awaitable);

        beast::error_code ec;
        stream.socket().shutdown(tcp::socket::shutdown_both, ec);

        co_return res;
    }
}

net::awaitable<http::response<http::string_body>>
HttpClient::get_async(const std::string &requestUri)
{
    auto parts = parse_url(requestUri);
    co_return co_await send_async(parts);
}

net::awaitable<std::string>
HttpClient::get_string_async(const std::string &requestUri)
{
    try
    {
        auto parts = parse_url(requestUri);
        http::response<http::string_body> res = co_await send_async(parts);

        co_return res.body();
    }
    catch (const std::exception &e)
    {
        co_return std::string("Error: ") + e.what();
    }
}

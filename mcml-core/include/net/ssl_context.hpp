#ifndef __SSL_CONTEXT_H__
#define __SSL_CONTEXT_H__

#include <boost/beast/core.hpp>
#include <boost/beast/ssl.hpp>
#include <boost/asio/ssl.hpp>

namespace net = boost::asio;
namespace ssl = net::ssl;

class SslContext
{
public:
    static SslContext &Instance() { return instance_; }

    ssl::context &get_context() { return ctx_; }

private:
    static SslContext instance_;

    SslContext();

    ssl::context ctx_;
};

#endif // __SSL_CONTEXT_H__
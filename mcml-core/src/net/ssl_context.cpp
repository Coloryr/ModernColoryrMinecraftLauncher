#include "net/ssl_context.hpp"

SslContext::SslContext() : ctx_(ssl::context::tlsv12_client)
{
    ctx_.set_default_verify_paths();
    ctx_.set_verify_mode(ssl::verify_peer);
}
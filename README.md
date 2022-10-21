# Ratelimit server


This is currently a POC / test project of a ratelimit library and server usable with the memcached protocol


## Warning

This is currently pre-alpha stuff, configuration settings will most probably change/break in the future

### Build

A simple `cargo build` should be enough


### Configuration

The configuration is currently used via `development.toml` and allows to set desired default ratelimit


### Server

The servers defaults listen on the port memcached port (11211), the only exposed command is `incr`

**Warning** the responses are **reversed**, a `0` means success while a `1` means that the limit was reached.
This is to allow memcache clients to ignore an unreachable / unresponsive server by default, for example:

````
if client.incr("client-${remote_addr}", timeout=0.02):
   return HTTP429()
```

The server allows per-client "custom" ratelimits using a pattern, so multiple clients might each have their own setup

For example:

- `incr something` will use the default limit set in the configuration, with the "something" as the key
- `incr 3000/3600_other` will limit "other" to 3000 queries per hour
- `incr 100/60_other` will  limit "other" to 100 queries per minute

All limits are independants of each other


```
% nc -v localhost 11211

incr foo
0
incr foo
1
incr bar
0
incr 1/1_foo
1
incr 1/1_foo
0
incr 80/60_something
0

```
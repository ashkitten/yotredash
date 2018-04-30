#include <alsa/asoundlib.h>
#include <alsa/error.h>
#include <jack/jack.h>

typedef void (*rust_callback)(uint8_t, const char*, const char*);
rust_callback rust_log;

void alsa_error_handler(
    __attribute__((unused)) const char *file,
    __attribute__((unused)) int line,
    __attribute__((unused)) const char *function,
    __attribute__((unused)) int err,
    const char *fmt,
    ...
) {
    const int len = 1024;
    char msg[len];

    va_list argptr;
    va_start(argptr, fmt);
    vsnprintf(msg, len - 1, fmt, argptr);
    va_end(argptr);

    rust_log(3, "yotredash::alsa", msg);
}

void jack_info_handler(const char *msg) {
    rust_log(2, "yotredash::jack", msg);
}

void jack_error_handler(const char *msg) {
    rust_log(3, "yotredash::jack", msg);
}

void set_message_handler(rust_callback callback) {
    rust_log = callback;

    snd_lib_error_set_handler(alsa_error_handler);
    jack_set_info_function(jack_info_handler);
    jack_set_error_function(jack_error_handler);
}

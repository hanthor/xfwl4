#include <stdarg.h>
#include <libseat.h>

#include <stdio.h>
#include <stdlib.h>

typedef void LogHandler(enum libseat_log_level level, const char *msg);

static LogHandler *current_log_handler = NULL;

static void formatter_handler(enum libseat_log_level level, const char *fmt, va_list args)
{
    if (current_log_handler == NULL)
    {
        return;
    }

    if (level > LIBSEAT_LOG_LEVEL_LAST)
    {
        level = LIBSEAT_LOG_LEVEL_LAST;
    }

    va_list argcopy;
    va_copy(argcopy, args);
    int length = vsnprintf(NULL, 0, fmt, argcopy);
    va_end(argcopy);

    if (length < 0)
    {
        return;
    }

    // +1 for null terminator
    length += 1;

    char *buffer = malloc(length);
    if (buffer == NULL)
    {
        return;
    }

    vsnprintf(buffer, length, fmt, args);

    current_log_handler(level, buffer);

    free(buffer);
}

void init_preformatted_log_handler(LogHandler *handler)
{
    current_log_handler = handler;
    libseat_set_log_handler(formatter_handler);
}

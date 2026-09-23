# syntax=docker/dockerfile:1.7
FROM composer:2.8@sha256:0d264a0f1e5be23ba363447768df7b30c33d542711ea12e37770ed7b13bf4eaa AS composer_bin
FROM php:8.5.6-fpm-bookworm@sha256:1afa03a1445c747fc2448219aa136727b7365a9566a896a4c0d05968093e4bab

RUN apt-get update \
 && apt-get install -y --no-install-recommends gcc git libc6-dev libicu-dev libonig-dev libzip-dev unzip \
 && docker-php-ext-install -j"$(nproc)" intl mbstring pcntl pdo_mysql \
 && rm -rf /var/lib/apt/lists/*
COPY --from=composer_bin /usr/bin/composer /usr/local/bin/composer
COPY --from=platform / /var/www/platform/
WORKDIR /var/www/platform
ENV APP_ENV=local APP_DEBUG=false LOG_CHANNEL=stderr COMPOSER_ALLOW_SUPERUSER=1
RUN composer install --no-dev --prefer-dist --no-interaction --no-progress --optimize-autoloader \
 && mkdir -p storage/framework/cache storage/framework/sessions storage/framework/views storage/logs bootstrap/cache \
 && chown -R www-data:www-data storage bootstrap/cache

RUN cat > /tmp/wp5_fsync_fault.c <<'C'
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

typedef int (*fsync_fn)(int);
int fsync(int fd) {
    static fsync_fn real_fsync;
    if (!real_fsync) real_fsync = (fsync_fn)dlsym(RTLD_NEXT, "fsync");
    const char *mode = getenv("WP5_FSYNC_FAULT");
    struct stat st;
    if (mode && fstat(fd, &st) == 0) {
        if ((strcmp(mode, "file") == 0 && S_ISREG(st.st_mode)) ||
            (strcmp(mode, "directory") == 0 && S_ISDIR(st.st_mode))) {
            errno = EIO;
            return -1;
        }
    }
    return real_fsync(fd);
}
C
RUN cc -shared -fPIC -O2 -Wall -Wextra -Werror -o /opt/wp5-libfsync-fault.so /tmp/wp5_fsync_fault.c -ldl \
 && rm /tmp/wp5_fsync_fault.c \
 && printf '%s\n' 'clear_env = no' 'catch_workers_output = yes' > /usr/local/etc/php-fpm.d/zz-wp5-s3a.conf

EXPOSE 9000
CMD ["php-fpm", "-F"]

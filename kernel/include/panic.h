#ifndef PANIC_H
#define PANIC_H

void panic(const char *file, int line, const char *msg);
#define PANIC(msg) panic(__FILE__, __LINE__, (msg))

#endif

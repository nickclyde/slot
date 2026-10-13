/*
 * slotpress: press buttons on the RG SP / RG35XXSP from a shell, for driving
 * slot over adb during development. Writes input events straight into the
 * existing /dev/input nodes, so slot (which opens its nodes once at boot and
 * never grabs them) sees them exactly like real presses. Not shipped on cards.
 *
 *   slotpress a                 tap A
 *   slotpress down down a       tap Down twice, then A
 *   slotpress a:800             hold A for 800 ms
 *   slotpress menu+x            press Menu and X together
 *   slotpress wait:500 b        wait half a second, then tap B
 */
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define TAP_MS 80
#define GAP_MS 120
#define MAX_CHORD 4

struct button {
    const char *name;
    unsigned short type;
    unsigned short code;
    int value;
};

static const struct button BUTTONS[] = {
    {"a", EV_KEY, 0x130, 1},
    {"b", EV_KEY, 0x131, 1},
    {"y", EV_KEY, 0x132, 1},
    {"x", EV_KEY, 0x133, 1},
    {"l1", EV_KEY, 0x134, 1},
    {"r1", EV_KEY, 0x135, 1},
    {"select", EV_KEY, 0x136, 1},
    {"start", EV_KEY, 0x137, 1},
    {"menu", EV_KEY, 0x138, 1},
    {"l2", EV_KEY, 0x13a, 1},
    {"r2", EV_KEY, 0x13b, 1},
    {"volup", EV_KEY, 115, 1},
    {"voldown", EV_KEY, 114, 1},
    {"power", EV_KEY, 116, 1},
    {"up", EV_ABS, ABS_HAT0Y, -1},
    {"down", EV_ABS, ABS_HAT0Y, 1},
    {"left", EV_ABS, ABS_HAT0X, -1},
    {"right", EV_ABS, ABS_HAT0X, 1},
};

#define N_BUTTONS (sizeof BUTTONS / sizeof BUTTONS[0])

static void sleep_ms(long ms) {
    struct timespec t = {ms / 1000, (ms % 1000) * 1000000L};
    while (nanosleep(&t, &t) == -1 && errno == EINTR) {
    }
}

static int has_bit(const char *hex_words, unsigned bit) {
    char buf[1024];
    strncpy(buf, hex_words, sizeof buf - 1);
    buf[sizeof buf - 1] = 0;
    char *words[64], *save;
    int n = 0;
    for (char *w = strtok_r(buf, " \n", &save); w && n < 64; w = strtok_r(NULL, " \n", &save)) {
        words[n++] = w;
    }
    unsigned per = sizeof(long) * 8;
    int index = n - 1 - (int)(bit / per);
    if (index < 0) {
        return 0;
    }
    unsigned long word = strtoul(words[index], NULL, 16);
    return (word >> (bit % per)) & 1;
}

static int node_for(const struct button *b) {
    const char *cap = b->type == EV_KEY ? "key" : "abs";
    DIR *dir = opendir("/sys/class/input");
    if (!dir) {
        return -1;
    }
    struct dirent *e;
    int fd = -1;
    while (fd < 0 && (e = readdir(dir))) {
        if (strncmp(e->d_name, "event", 5) != 0) {
            continue;
        }
        char path[512], words[1024] = {0};
        snprintf(path, sizeof path, "/sys/class/input/%s/device/capabilities/%s", e->d_name, cap);
        FILE *f = fopen(path, "r");
        if (!f) {
            continue;
        }
        size_t got = fread(words, 1, sizeof words - 1, f);
        fclose(f);
        words[got] = 0;
        if (has_bit(words, b->code)) {
            snprintf(path, sizeof path, "/dev/input/%s", e->d_name);
            fd = open(path, O_WRONLY);
        }
    }
    closedir(dir);
    return fd;
}

static void emit(int fd, unsigned short type, unsigned short code, int value) {
    struct input_event ev[2];
    memset(ev, 0, sizeof ev);
    ev[0].type = type;
    ev[0].code = code;
    ev[0].value = value;
    ev[1].type = EV_SYN;
    ev[1].code = SYN_REPORT;
    if (write(fd, ev, sizeof ev) != (ssize_t)sizeof ev) {
        perror("slotpress: write");
        exit(1);
    }
}

static const struct button *lookup(const char *name) {
    for (size_t i = 0; i < N_BUTTONS; i++) {
        if (strcmp(BUTTONS[i].name, name) == 0) {
            return &BUTTONS[i];
        }
    }
    return NULL;
}

static void usage(void) {
    fprintf(stderr, "usage: slotpress STEP...\n"
                    "  STEP is BUTTON, BUTTON:MS (hold), A+B[:MS] (chord) or wait:MS\n"
                    "  buttons:");
    for (size_t i = 0; i < N_BUTTONS; i++) {
        fprintf(stderr, " %s", BUTTONS[i].name);
    }
    fprintf(stderr, "\n");
    exit(2);
}

static void step(char *arg) {
    long hold = TAP_MS;
    char *colon = strchr(arg, ':');
    if (colon) {
        *colon = 0;
        char *end;
        hold = strtol(colon + 1, &end, 10);
        if (*end || hold < 0) {
            usage();
        }
    }
    if (strcmp(arg, "wait") == 0) {
        sleep_ms(hold);
        return;
    }
    const struct button *chord[MAX_CHORD];
    int fds[MAX_CHORD], n = 0;
    char *save;
    for (char *name = strtok_r(arg, "+", &save); name; name = strtok_r(NULL, "+", &save)) {
        if (n == MAX_CHORD) {
            usage();
        }
        const struct button *b = lookup(name);
        if (!b) {
            fprintf(stderr, "slotpress: unknown button %s\n", name);
            usage();
        }
        int fd = node_for(b);
        if (fd < 0) {
            fprintf(stderr, "slotpress: no input node takes %s\n", name);
            exit(1);
        }
        chord[n] = b;
        fds[n++] = fd;
    }
    for (int i = 0; i < n; i++) {
        emit(fds[i], chord[i]->type, chord[i]->code, chord[i]->value);
    }
    sleep_ms(hold);
    for (int i = n - 1; i >= 0; i--) {
        emit(fds[i], chord[i]->type, chord[i]->code, 0);
        close(fds[i]);
    }
    sleep_ms(GAP_MS);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        usage();
    }
    for (int i = 1; i < argc; i++) {
        step(argv[i]);
    }
    return 0;
}

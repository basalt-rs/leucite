#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    for (int i = 1; i < 10; ++i) {
        char *data = malloc(1 * 1000 * 1000);
        if (data == NULL) {
            fprintf(stderr, "[ERR] Out of memory\n");
            return 69;
        }

        char n[] = { 'H', 'i', ' ', '0' + i, '\0' };

        memcpy(data, n, sizeof(n));
        printf("%s\n", data);
    }
    return 0;
}

CC = gcc
CFLAGS = -shared -fPIC -O2 -I../include
LDFLAGS = -lm

MODULES = astra.so
SRCS = $(MODULES:.so=.c)

.PHONY: all clean

all: $(MODULES)

astra.so: astra.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f $(MODULES)

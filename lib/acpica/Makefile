
# Makefile inputs
#   BINDIR: Where to put the generated binaries
#	LIB: Path to the target library
#   ARCH: CPU 
# 	CFLAGS: Flags to pass to C

.PHONY: all dirs

SRCDIR := acpica

SRC := $(wildcard $(SRCDIR)/*.c)
OBJ := $(subst $(SRCDIR),$(BINDIR),$(SRC:.c=.o))

TARGET := $(ARCH)-none-elf

CC ?= clang
AR ?= llvm-ar

all: dirs $(LIB)
	@echo cargo:rerun-if-changed=$(SRC)

$(LIB): $(OBJ)
	$(AR) -crs $@ $^

$(BINDIR)/%.o: $(SRCDIR)/%.c
	$(CC) -c -o $@ $^ -target $(TARGET)

dirs:
	mkdir -p $(BINDIR)

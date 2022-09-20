
.PHONY: doc kernel build clean qemu run k210 flash

TARGET      := riscv64imac-unknown-none-elf
MODE        ?= release
MODE_FLAG	:= --target=riscv64imac-unknown-none-elf
KERNEL_FILE := target/$(TARGET)/$(MODE)/os
BIN_FILE    := target/$(TARGET)/$(MODE)/kernel.bin
DEBUG_FILE  ?= $(KERNEL_FILE)
FEATURES	?= 

OBJDUMP     := rust-objdump --arch-name=riscv64
OBJCOPY     := rust-objcopy --binary-architecture=riscv64

FS_IMG := fs.img

# BOARD
BOOTLOADER := bootloader/rustsbi-qemu.bin
# BOOTLOADER := bootloader/opensbi-qemu.bin
BOOTLOADER_K210 := bootloader/rustsbi-k210.bin

K210-SERIALPORT	= /dev/ttyUSB0
K210-BURNER	= tools/kflash.py

LINK_FILE_DIR = kernel/src

ifeq ($(MODE), release)
MODE_FLAG += --release
endif

ifeq ($(VERBOSE), 0)
FEATURES += not_debug
endif

.PHONY: all doc kernel build clean qemu run k210 flash

all: qemu
	cp $(BOOTLOADER) sbi-qemu
	cp $(KERNEL_FILE) kernel-qemu

#all: k210
#	@cp $(BIN_FILE) os.bin


build: kernel $(BIN_FILE)

qemu:
ifeq ($(MODE), release)
	echo "release"
endif
	@cp $(LINK_FILE_DIR)/linker-qemu.ld $(LINK_FILE_DIR)/linker.ld
	@RUSTFLAGS="-Clink-arg=-T$(LINK_FILE_DIR)/linker.ld" cargo build $(MODE_FLAG) --features "board_qemu $(FEATURES)"
#	--offline
	@rm $(LINK_FILE_DIR)/linker.ld
	$(OBJCOPY) $(KERNEL_FILE) --strip-all -O binary $(BIN_FILE)

asm:
	@$(OBJDUMP) -d $(KERNEL_FILE) | less

# 清理编译出的文件
clean:
	@cargo clean

k210: 
	@cp $(LINK_FILE_DIR)/linker-k210.ld $(LINK_FILE_DIR)/linker.ld
	RUSTFLAGS="-Clink-arg=-T$(LINK_FILE_DIR)/linker.ld" cargo build $(MODE_FLAG) --features "board_k210 $(FEATURES)" --offline
	@rm $(LINK_FILE_DIR)/linker.ld
	$(OBJCOPY) $(KERNEL_FILE) --strip-all -O binary $(BIN_FILE)
	@cp $(BOOTLOADER_K210) $(BOOTLOADER_K210).copy
	@dd if=$(BIN_FILE) of=$(BOOTLOADER_K210).copy bs=131072 seek=1
	@mv $(BOOTLOADER_K210).copy $(BIN_FILE)

flash: k210
	(which $(K210-BURNER)) || (cd .. && git clone https://hub.fastgit.xyz/sipeed/kflash.py.git && mv kflash.py tools)
	@sudo chmod 777 $(K210-SERIALPORT)
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(BIN_FILE)
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200

run: qemu
	@cp fs-origin.img fs.img
	@qemu-system-riscv64 \
            -machine virt \
            -bios $(BOOTLOADER) \
            -device loader,file=$(BIN_FILE),addr=0x80200000 \
			-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
        	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
			-kernel $(BIN_FILE) \
			-nographic \
			-smp 4 -m 128m
	@rm fs.img
# qemu-system-riscv64 -machine virt -bios sbi-qemu -device loader,file=kernel-qemu,addr=0x80200000 -drive file=fs.img,if=none,format=raw,id=x0 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 -kernel kernel-qemu -nographic -smp 4 


debug: qemu
	@cp fs-origin.img fs.img
	@tmux new-session -d \
	"qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -drive file=$(FS_IMG),if=none,format=raw,id=x0 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 -device loader,file=$(BIN_FILE),addr=0x80200000 -s -S && echo '按任意键继续' && read -n 1" && \
	tmux split-window -h "riscv64-elf-gdb -ex 'file $(DEBUG_FILE)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
	tmux -2 attach-session -d
	@rm fs.img

gdb:
	riscv64-elf-gdb \
        -ex 'file $(DEBUG_FILE)' \
        -ex 'set arch riscv:rv64' \
        -ex 'target remote localhost:1234'

hexdump:
	hexdump $(FS_IMG) -C -s 0x10F200

coredump:
	cd kernel && make -f makefile FS_IMG=../$(FS_IMG) coredump || exit 1;

objdump:
	rust-objdump --arch-name=riscv64 -d $(KERNEL_FILE)

clean:
	rm $(KERNEL_FILE) $(BIN_FILE)

fs-img: 
	@rm -f $(FS_IMG)
	@dd if=/dev/zero of=$(FS_IMG) count=81920 bs=512	# 40M
	@mkfs.vfat $(FS_IMG) -F 32
docker:
	docker run --rm -it -v ${PWD}:/mnt -w /mnt qemu:4.2.1 bash

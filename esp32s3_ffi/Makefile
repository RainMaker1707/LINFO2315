all: clean build flash

ffi/target/xtensa-esp32s3-espidf/release/libffi.a:
	cd ffi && cargo build --release

build/my_freertos_ffi.elf: ffi/target/xtensa-esp32s3-espidf/release/libffi.a
	idf.py build

build: build/my_freertos_ffi.elf

flash:
	idf.py flash monitor

clean:
	cargo clean --manifest-path=ffi/Cargo.toml
	idf.py clean

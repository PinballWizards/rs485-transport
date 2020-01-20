all:
	cargo build --release
	arm-none-eabi-objcopy -O binary target/thumbv6m-none-eabi/release/rs485-transport target/thumbv6m-none-eabi/release/rs485-transport.bin
	uf2conv-rs target/thumbv6m-none-eabi/release/rs485-transport.bin
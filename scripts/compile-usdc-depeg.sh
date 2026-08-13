echo "Compiling following file:"
cat examples/usdc_depeg.lua
cargo run -p proveno-compiler -- examples/usdc_depeg.lua /tmp/compiled.json

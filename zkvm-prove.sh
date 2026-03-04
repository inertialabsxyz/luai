cd compiler
cargo run -- ../examples/prover.lua /tmp/compiled.json
cd ../prover
cargo run -- /tmp/compiled.json /tmp/dry_result.json
cd ../openvm
echo "encode inputs for openvm"
cargo run --bin luai-openvm-encoder -- /tmp/compiled.json /tmp/dry_result.json
echo "run circuit"
cargo openvm run --bin luai-openvm --input /tmp/openvm-1.json

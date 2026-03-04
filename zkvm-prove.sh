cd compiler
cargo run -- ../examples/prover.lua /tmp/compiled.json
cd ../prover
cargo run -- /tmp/compiled.json /tmp/dry_result.json
cd ../openvm
# echo "keygen..."
# cargo openvm keygen
# echo "commit..."
# cargo openvm commit
echo "encode inputs"
cargo run --bin luai-openvm-encoder -- /tmp/compiled.json /tmp/dry_result.json
echo "run circuit"
cargo openvm run --bin luai-openvm --input /tmp/openvm-1.json

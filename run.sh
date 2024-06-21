./pid.sh
cargo build --release
nohup ./target/release/spotify-alarm-cli $@ >output.log 2>&1 &
echo kill -9 $! >pid.sh
echo "ps aux | grep $!" >is-alive.sh
chmod +x pid.sh is-alive.sh
ps aux | grep spotify-alarm-cli

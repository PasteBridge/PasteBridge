#!/system/bin/sh
# Minimal HTTP server: just serves a static response, no printf tricks.
PORT=${1:-18792}
LOG=/data/local/tmp/pb_srv2.log
PIDFILE=/data/local/tmp/pb_srv2.pid
RESP=/data/local/tmp/pb_response2.bin

# Write the response once (no leading whitespace).
cat > "$RESP" <<'EOF'
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 441
Access-Control-Allow-Origin: *
Connection: close

[{"id":99,"content_type":"text","content_text":"HELLO-FROM-ANDROID","content_hash":"abc99def","mime_type":null,"file_size":null,"width":null,"height":null,"source_ip":null,"created_at":1700000000000,"is_favorite":false},{"id":100,"content_type":"text","content_text":"test-from-phone-2","content_hash":"abc100def","mime_type":null,"file_size":null,"width":null,"height":null,"source_ip":null,"created_at":1700000060000,"is_favorite":false}]
EOF

rm -f "$LOG" "$PIDFILE"
echo "$$" > "$PIDFILE"
echo "PB-dummy-server-v2 on port $PORT (pid $$)" > "$LOG"

# Outer loop: keep restarting nc on each connection.
while true; do
  cat "$RESP" | nc -l -p "$PORT" 2>>"$LOG"
  sleep 0.2
done
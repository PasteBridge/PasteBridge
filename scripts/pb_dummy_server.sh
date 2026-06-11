#!/system/bin/sh
# Persistent HTTP server: while loop restarts nc -l per connection.
# Pre-builds the response file; cat pipes it to nc for each accept.
LOG=/data/local/tmp/pb_srv.log
PIDFILE=/data/local/tmp/pb_srv.pid
PORT=${1:-18792}

HISTORY='[{"id":99,"content_type":"text","content_text":"HELLO-FROM-ANDROID","content_hash":"abc99def","mime_type":null,"file_size":null,"width":null,"height":null,"source_ip":null,"created_at":1700000000000,"is_favorite":false},{"id":100,"content_type":"text","content_text":"test-from-phone-2","content_hash":"abc100def","mime_type":null,"file_size":null,"width":null,"height":null,"source_ip":null,"created_at":1700000060000,"is_favorite":false}]'
CLEN=$(printf '%s' "$HISTORY" | wc -c)
RESP=/data/local/tmp/pb_response.bin

# Build response file (CRLF line endings, exact Content-Length).
printf 'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: %d\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n%s' "$CLEN" "$HISTORY" > "$RESP"

rm -f "$LOG" "$PIDFILE"
echo "$$" > "$PIDFILE"
echo "PB-dummy-server starting on port $PORT (pid $$), response bytes=$(wc -c < $RESP)" >> "$LOG"

# Outer loop: keep restarting nc -l on each connection. The sleep avoids the
# TIME_WAIT window so the next bind succeeds.
while true; do
  cat "$RESP" | nc -l -p "$PORT" 2>>"$LOG"
  # Small delay so the kernel reaps the TIME_WAIT socket before next bind.
  sleep 0.3
done

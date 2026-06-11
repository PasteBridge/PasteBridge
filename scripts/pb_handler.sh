#!/system/bin/sh
# Per-connection HTTP handler. stdin/stdout are the socket.
# Drains the request, then writes a canned 200 + JSON response from a precomputed file.
LOG=/data/local/tmp/pb_srv.log
RESP=/data/local/tmp/pb_response.bin

# Consume request headers until empty line.
while IFS= read -r line; do
  case "$line" in
    "")
      break
      ;;
  esac
done
# If read returned EOF, just exit (client closed).
[ ! -r "$RESP" ] && exit 0

# cat sends the precomputed response bytes straight to the socket.
cat "$RESP"

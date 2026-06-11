#!/system/bin/sh
# 一次性 HTTP server: 每次 accept 后读一 HTTP request, 回 200 + JSON
BODY='[{"id":99,"content_type":"text","content_text":"HELLO-FROM-ANDROID","content_hash":"abc","created_at":1700000000,"is_favorite":false}]'
LEN=${#BODY}
while true; do
  {
    # 读 request (直到空行, 表示 HTTP header 结束)
    while read -r line; do
      [ -z "$line" ] && break
    done
    printf 'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: %d\r\nConnection: close\r\n\r\n%s' "$LEN" "$BODY"
  } | nc -l -p 18792
done

#!/system/bin/sh
# Find the process holding a TCP socket with given inode.
INODE=$1
[ -z "$INODE" ] && INODE=24555083
for pid in $(ls /proc/ 2>/dev/null | grep -E '^[0-9]+$'); do
  for fd in /proc/$pid/fd/*; do
    target=$(readlink $fd 2>/dev/null)
    if [ "$target" = "socket:[$INODE]" ]; then
      echo "PID=$pid CMD=$(cat /proc/$pid/cmdline 2>/dev/null | tr '\0' ' ')"
    fi
  done
done

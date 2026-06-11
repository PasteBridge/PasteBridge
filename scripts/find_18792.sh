#!/system/bin/sh
for pid in $(ls /proc/ 2>/dev/null | grep -E '^[0-9]+$'); do
  if ls /proc/$pid/fd 2>/dev/null | head -1 >/dev/null; then
    ls -la /proc/$pid/fd 2>/dev/null | grep -E "18792|socket" | sed "s|^|pid=$pid: |" | head -5
  fi
done | head -30

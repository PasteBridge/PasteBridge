#!/system/bin/sh
for pid in $(ps -ef | grep -E 'nc -l|android_dummy' | grep -v grep | awk '{print $2}'); do
  kill -9 $pid 2>/dev/null
done
sleep 0.5
echo "killed; remaining:"
ps -ef | grep -E 'nc -l|android_dummy' | grep -v grep || echo none

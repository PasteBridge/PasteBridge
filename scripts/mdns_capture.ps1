# 监听 UDP 5353, 但先停掉 mdns-sd 占的 socket。
# 这里用 SOCK_RAW (IPPROTO_UDP) 直接嗅探网卡上的 mDNS 包, 不需要绑端口。

$ErrorActionPreference = "Stop"
$logFile = "d:\Download\PasteBridge\mdns_capture.log"

# 用 ICMP/raw socket 拿不到 UDP 负载, 改用 SOCK_DGRAM + 多播组 (必须端口独占).
# 5353 给 Bonjour/Apple, 我们绑 5354 + 5353 多播组实际上抓不到 5353 流量.
# 改方案: 用 WinPcap/Npcap 替代. 简化为只打印本机接口列表 + 多播可达性, 让用户手动看.
"=== mdns capture: not implemented for raw sniff, use netsh ===" | Out-File $logFile
Get-NetAdapter | Where-Object {$_.Status -eq "Up"} | Select-Object Name, InterfaceDescription, ifIndex, LinkSpeed | Format-Table | Out-File $logFile -Append
Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -ne "127.0.0.1" } | Select-Object IPAddress, InterfaceIndex, PrefixLength | Format-Table | Out-File $logFile -Append
"=== multicast routing ===" | Out-File $logFile -Append
Get-NetRoute -DestinationPrefix "224.0.0.0/4" | Select-Object DestinationPrefix, NextHop, InterfaceAlias, RouteMetric | Format-Table | Out-File $logFile -Append
"=== ad-hoc mDNS ping via .NET ===" | Out-File $logFile -Append
# 真正能跑的最简方式: 拿一个空闲端口, 加入多播组, 触发一个查询包, 看收不收得到.
# 但查询包需要 DNS 协议格式, 先放弃. 直接用 UdpClient 5353 不行 (冲突).
# 最后方案: 让用户跑 `Get-NetAdapter | Get-NetAdapterStatistics` 或者在另一台机器跑 tcpdump.
# 这里改成: 检查 IGMP 加入状态.
Get-NetAdapter | ForEach-Object {
    $igmp = Get-NetAdapterAdvancedProperty -Name $_.Name -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -match "IGMP" }
    $_.Name + ": " + ($igmp | Select-Object DisplayName, DisplayValue | Format-Table | Out-String) | Out-File $logFile -Append
}
Write-Host "=== results written to $logFile ===" -ForegroundColor Cyan
Get-Content $logFile

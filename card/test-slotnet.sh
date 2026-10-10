#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/System/slotnet.sh"
TMP="$(mktemp -d)"
FAILS=0
PIDS=

cleanup() {
	for p in $PIDS $(cat "$TMP"/pids 2>/dev/null); do kill "$p" 2>/dev/null || true; done
	rm -rf "$TMP"
}
trap cleanup EXIT HUP INT TERM

fail() {
	echo "FAIL: $*"
	FAILS=$((FAILS + 1))
}

fake() {
	{
		echo '#!/bin/sh'
		echo "printf '%s\\n' \"\$*\" >> \"$TMP/$1.log\""
		cat
	} > "$TMP/bin/$1"
	chmod 755 "$TMP/bin/$1"
}

spawn_as() {
	echo "sleep 30 & p=\$!; echo \$p >> \"$TMP/pids\"; mkdir -p \"$TMP/proc/\$p\"; printf '$1' > \"$TMP/proc/\$p/cmdline\""
}

reset() {
	rm -rf "$TMP/net" "$TMP/run" "$TMP/ctrl" "$TMP/proc" "$TMP/bin" "$TMP"/*.log "$TMP/root"
	mkdir -p "$TMP/net/wlan0" "$TMP/run" "$TMP/ctrl" "$TMP/proc" "$TMP/bin" "$TMP/root/Config"
	echo "nameserver 9.9.9.9" > "$TMP/resolv.conf"
	printf 'ssid = Home Net\npsk = pa ss"word\n' > "$TMP/root/Config/wifi.txt"
	fake wpa_supplicant <<-EOF
		while [ \$# -gt 0 ]; do
			case "\$1" in
			-P) pidfile="\$2"; shift ;;
			-c) conf="\$2"; shift ;;
			esac
			shift
		done
		$(spawn_as 'wpa_supplicant\0-c\0slotnet.conf\0')
		echo \$p > "\$pidfile"
	EOF
	fake wpa_cli <<-'EOF'
		case "$*" in
		*ping*) [ -n "${FAKE_FOREIGN:-}" ] && echo PONG && exit 0; exit 1 ;;
		*status*) echo "wpa_state=${FAKE_STATE:-COMPLETED}" ;;
		esac
	EOF
	fake ip <<-'EOF'
		case "$*" in
		"-4 addr show dev wlan0") [ -n "${FAKE_ONLINE:-}" ] && echo "    inet 192.168.9.9/24 scope global wlan0" ;;
		"route show default") [ -n "${FAKE_ONLINE:-}" ] && echo "default via 192.168.9.1 dev wlan0" ;;
		esac
		exit 0
	EOF
	fake udhcpc <<-EOF
		while [ \$# -gt 0 ]; do
			case "\$1" in
			-p) pidfile="\$2"; shift ;;
			-s) hook="\$2"; shift ;;
			esac
			shift
		done
		$(spawn_as 'udhcpc\0-s\0/run/slotnet-dhcp.sh\0')
		echo \$p > "\$pidfile"
		ip=192.168.1.50 subnet=255.255.255.0 router="192.168.1.1 192.168.1.2" dns="1.1.1.1 8.8.8.8" "\$hook" bound
	EOF
}

run() {
	PATH="$TMP/bin:$PATH" SLOT_ROOT="$TMP/root" \
		AGS_NET_SYS="$TMP/net" AGS_RUN="$TMP/run" AGS_PROC="$TMP/proc" AGS_CTRL_DIR="$TMP/ctrl" \
		AGS_RESOLV="$TMP/resolv.conf" AGS_WPA_SUPPLICANT="$TMP/bin/wpa_supplicant" \
		AGS_WPA_CLI="$TMP/bin/wpa_cli" AGS_IP="$TMP/bin/ip" \
		AGS_UDHCPC="${UDHCPC:-$TMP/bin/udhcpc}" AGS_BUSYBOX="${BUSYBOX:-$TMP/bin/busybox}" \
		AGS_WIFI_WAIT_S=1 AGS_DHCP_WAIT_S=1 \
		sh "$SCRIPT" "$@"
}

alive() {
	kill -0 "$(cat "$1")" 2>/dev/null
}

reset
rm "$TMP/root/Config/wifi.txt"
run up && rc=0 || rc=$?
[ "$rc" = 3 ] || fail "no config exited $rc, want 3"
[ -f "$TMP/wpa_supplicant.log" ] && fail "no config still started wpa_supplicant"

reset
: > "$TMP/run/slotlink.session"
run up && rc=0 || rc=$?
[ "$rc" = 4 ] || fail "link play exited $rc, want 4"
[ -f "$TMP/wpa_supplicant.log" ] && fail "started wpa_supplicant during link play"

reset
(export FAKE_ONLINE=1; run up) && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "already online exited $rc"
[ -f "$TMP/wpa_supplicant.log" ] && fail "restarted wifi that was already online"

reset
run up && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "up exited $rc"
conf="$TMP/run/slotnet.conf"
grep -q "ssid=$(printf 'Home Net' | od -An -tx1 | tr -d ' \n')" "$conf" || fail "ssid not hex encoded"
grep -q 'psk="pa ss"word"' "$conf" || fail "passphrase not quoted"
grep -q "ctrl_interface=$TMP/ctrl" "$conf" || fail "not on the shared control socket"
[ "$(stat -c %a "$conf" 2>/dev/null || stat -f %Lp "$conf")" = 600 ] || fail "supplicant conf readable by others"
grep -q -- "-P $TMP/run/slotnet-wpa.pid" "$TMP/wpa_supplicant.log" || fail "supplicant has no pidfile"
grep -q -- "-s $TMP/run/slotnet-dhcp.sh" "$TMP/udhcpc.log" || fail "udhcpc not using our hook"
grep -q 'addr add 192.168.1.50/24 dev wlan0' "$TMP/ip.log" || fail "lease not addressed"
grep -q 'route replace default via 192.168.1.1 dev wlan0' "$TMP/ip.log" || fail "no default route"
[ "$(cat "$TMP/resolv.conf")" = "nameserver 1.1.1.1
nameserver 8.8.8.8" ] || fail "dns not written"
wpa="$TMP/run/slotnet-wpa.pid"
dhcp="$TMP/run/slotnet-dhcp.pid"
alive "$wpa" || fail "supplicant not running"
alive "$dhcp" || fail "udhcpc not running"

run down && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "down exited $rc"
sleep 0.2 2>/dev/null || sleep 1
grep -q 'addr del 192.168.1.50/24 dev wlan0' "$TMP/ip.log" || fail "down left the lease address"
grep -q 'route del default via 192.168.1.1 dev wlan0' "$TMP/ip.log" || fail "down left the route"
grep -q 'flush' "$TMP/ip.log" && fail "down flushed addresses it does not own"
grep -q 'terminate' "$TMP/wpa_cli.log" && fail "down terminated a supplicant by socket"
[ "$(cat "$TMP/resolv.conf")" = "nameserver 9.9.9.9" ] || fail "dns not restored"
[ -f "$TMP/run/slotnet.lease" ] && fail "lease file left behind"
[ -f "$conf" ] && fail "supplicant conf left behind"

reset
sleep 30 &
foreign=$!
PIDS="$PIDS $foreign"
mkdir -p "$TMP/proc/$foreign"
printf 'wpa_supplicant\0-c\0/run/slotlink-sta.conf\0' > "$TMP/proc/$foreign/cmdline"
echo "$foreign" > "$TMP/run/slotnet-wpa.pid"
run down
kill -0 "$foreign" 2>/dev/null || fail "down killed link play's supplicant"

reset
printf 'ssid=Cafe\n' > "$TMP/root/Config/wifi.txt"
run up >/dev/null 2>&1 || true
grep -q 'key_mgmt=NONE' "$TMP/run/slotnet.conf" || fail "open network not key_mgmt=NONE"
run down

reset
printf 'ssid=Home\npsk=%s\n' 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef > "$TMP/root/Config/wifi.txt"
run up >/dev/null 2>&1 || true
grep -q 'psk=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef$' "$TMP/run/slotnet.conf" \
	|| fail "raw psk was quoted"
run down

reset
mv "$TMP/bin/udhcpc" "$TMP/bin/real-udhcpc"
fake busybox <<-EOF
	case "\$1" in
	--list) echo ash; echo udhcpc ;;
	udhcpc) shift; exec "$TMP/bin/real-udhcpc" "\$@" ;;
	esac
EOF
(export UDHCPC="$TMP/bin/udhcpc"; run up) && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "busybox udhcpc exited $rc"
grep -q '^udhcpc -i wlan0' "$TMP/busybox.log" || fail "busybox udhcpc not used"
(export UDHCPC="$TMP/bin/udhcpc"; run down)

reset
rm "$TMP/bin/udhcpc"
(export UDHCPC="$TMP/bin/udhcpc"; run up) && rc=0 || rc=$?
[ "$rc" = 5 ] || fail "no dhcp client exited $rc, want 5"
run down

reset
printf 'ssid=Home\npsk=secret\naddress=10.0.0.7/24\ngateway=10.0.0.1\ndns=10.0.0.1\n' > "$TMP/root/Config/wifi.txt"
run up && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "static exited $rc"
grep -q 'addr add 10.0.0.7/24 dev wlan0' "$TMP/ip.log" || fail "static address not set"
grep -q 'route replace default via 10.0.0.1 dev wlan0' "$TMP/ip.log" || fail "static gateway not set"
[ -f "$TMP/udhcpc.log" ] && fail "static config still ran dhcp"
run down

reset
(export FAKE_STATE=SCANNING; run up) && rc=0 || rc=$?
[ "$rc" = 1 ] || fail "no association exited $rc, want 1"
[ -f "$TMP/udhcpc.log" ] && fail "ran dhcp without associating"
run down

reset
(export FAKE_FOREIGN=1; run up) && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "existing supplicant exited $rc"
[ -f "$TMP/wpa_supplicant.log" ] && fail "started a second supplicant"
run down

grep -q 'slotlink.session' "$HERE/System/slotlink.sh" || fail "slotlink.sh no longer marks link play"

[ "$FAILS" = 0 ] || exit 1
echo "slotnet: ok"

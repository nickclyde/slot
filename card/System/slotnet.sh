#!/bin/sh
set -u

ROOT="${SLOT_ROOT:-/mnt/sdcard}"
CONF="${AGS_WIFI_CONF:-$ROOT/Config/wifi.txt}"
NET="${AGS_NET_SYS:-/sys/class/net}"
RUN="${AGS_RUN:-/run}"
PROC="${AGS_PROC:-/proc}"
CTRL_DIR="${AGS_CTRL_DIR:-/var/run/wpa_supplicant}"
RESOLV="${AGS_RESOLV:-/etc/resolv.conf}"

WPA_SUPPLICANT="${AGS_WPA_SUPPLICANT:-wpa_supplicant}"
WPA_CLI="${AGS_WPA_CLI:-wpa_cli}"
IP="${AGS_IP:-ip}"
UDHCPC="${AGS_UDHCPC:-udhcpc}"
BUSYBOX="${AGS_BUSYBOX:-busybox}"

ASSOC_WAIT_S="${AGS_WIFI_WAIT_S:-20}"
DHCP_WAIT_S="${AGS_DHCP_WAIT_S:-15}"

LINK_MARK="$RUN/slotlink.session"
SUP_CONF="$RUN/slotnet.conf"
SUP_PID="$RUN/slotnet-wpa.pid"
DHCP_PID="$RUN/slotnet-dhcp.pid"
DHCP_HOOK="$RUN/slotnet-dhcp.sh"
LEASE="$RUN/slotnet.lease"
ROUTE="$RUN/slotnet.route"
RESOLV_SAVED="$RUN/slotnet-resolv.saved"
RESOLV_BOUND="$RUN/slotnet-resolv.bound"
SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"

conf() {
	sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*//p" "$CONF" 2>/dev/null | tail -n 1 | sed 's/[[:space:]]*$//'
}

linked() {
	[ -e "$LINK_MARK" ]
}

online() {
	$IP -4 addr show dev wlan0 2>/dev/null | grep -q 'inet ' \
		&& $IP route show default 2>/dev/null | grep -q 'dev wlan0'
}

nap() {
	sleep 0.25 2>/dev/null || sleep 1
}

ours() {
	[ -f "$1" ] || return 1
	pid="$(cat "$1" 2>/dev/null)"
	[ -n "$pid" ] || return 1
	tr '\0' ' ' < "$PROC/$pid/cmdline" 2>/dev/null | grep -q "$2"
}

stop_ours() {
	if ours "$1" "$2"; then
		kill "$(cat "$1")" 2>/dev/null || true
	fi
	/bin/rm -f "$1"
}

hex() {
	printf '%s' "$1" | od -An -tx1 | tr -d ' \n'
}

prefix_of() {
	bits=0
	for octet in $(echo "$1" | tr '.' ' '); do
		case "$octet" in
		255) bits=$((bits + 8)) ;;
		254) bits=$((bits + 7)) ;;
		252) bits=$((bits + 6)) ;;
		248) bits=$((bits + 5)) ;;
		240) bits=$((bits + 4)) ;;
		224) bits=$((bits + 3)) ;;
		192) bits=$((bits + 2)) ;;
		128) bits=$((bits + 1)) ;;
		esac
	done
	echo "$bits"
}

set_dns() {
	[ -n "$1" ] || return 0
	if [ ! -e "$RESOLV_SAVED" ]; then
		cp "$RESOLV" "$RESOLV_SAVED" 2>/dev/null || : > "$RESOLV_SAVED"
	fi
	{
		for server in $1; do echo "nameserver $server"; done
	} > "$RUN/slotnet-resolv.conf"
	if cat "$RUN/slotnet-resolv.conf" > "$RESOLV" 2>/dev/null; then
		return 0
	fi
	if [ ! -e "$RESOLV_BOUND" ] && mount --bind "$RUN/slotnet-resolv.conf" "$RESOLV" 2>/dev/null; then
		: > "$RESOLV_BOUND"
	fi
}

restore_dns() {
	if [ -e "$RESOLV_BOUND" ]; then
		umount "$RESOLV" 2>/dev/null || true
		/bin/rm -f "$RESOLV_BOUND"
	elif [ -e "$RESOLV_SAVED" ]; then
		cat "$RESOLV_SAVED" > "$RESOLV" 2>/dev/null || true
	fi
	/bin/rm -f "$RESOLV_SAVED" "$RUN/slotnet-resolv.conf"
}

address() {
	if [ -f "$LEASE" ] && [ "$(cat "$LEASE")" != "$1" ]; then
		$IP addr del "$(cat "$LEASE")" dev wlan0 2>/dev/null || true
	fi
	$IP addr add "$1" dev wlan0 2>/dev/null || true
	echo "$1" > "$LEASE"
	if [ -n "$2" ]; then
		$IP route replace default via "$2" dev wlan0 2>/dev/null || true
		echo "$2" > "$ROUTE"
	fi
	set_dns "$3"
}

forget_address() {
	if [ -f "$ROUTE" ]; then
		$IP route del default via "$(cat "$ROUTE")" dev wlan0 2>/dev/null || true
		/bin/rm -f "$ROUTE"
	fi
	if [ -f "$LEASE" ]; then
		$IP addr del "$(cat "$LEASE")" dev wlan0 2>/dev/null || true
		/bin/rm -f "$LEASE"
	fi
}

dhcp_event() {
	case "${1:-}" in
	bound | renew)
		[ -n "${ip:-}" ] || return 0
		bits="${mask:-}"
		[ -n "$bits" ] || bits="$(prefix_of "${subnet:-255.255.255.0}")"
		address "$ip/$bits" "$(echo "${router:-}" | cut -d' ' -f1)" "${dns:-}"
		;;
	deconfig)
		forget_address
		;;
	esac
	return 0
}

dhcp_client() {
	if command -v "$UDHCPC" >/dev/null 2>&1; then
		echo "$UDHCPC"
	elif command -v "$BUSYBOX" >/dev/null 2>&1 && "$BUSYBOX" --list 2>/dev/null | grep -qx udhcpc; then
		echo "$BUSYBOX udhcpc"
	fi
}

supplicant_conf() {
	ssid="$(conf ssid)"
	psk="$(conf psk)"
	{
		echo "ctrl_interface=$CTRL_DIR"
		echo 'network={'
		echo "	ssid=$(hex "$ssid")"
		echo '	scan_ssid=1'
		if [ -z "$psk" ]; then
			echo '	key_mgmt=NONE'
		elif echo "$psk" | grep -Eq '^[0-9a-fA-F]{64}$'; then
			echo '	key_mgmt=WPA-PSK'
			echo "	psk=$psk"
		else
			echo '	key_mgmt=WPA-PSK'
			echo "	psk=\"$psk\""
		fi
		echo '}'
	} > "$SUP_CONF"
	/bin/chmod 600 "$SUP_CONF"
}

associated() {
	end=$(($(date +%s) + ASSOC_WAIT_S))
	while :; do
		linked && return 4
		state=$($WPA_CLI -p "$CTRL_DIR" -i wlan0 status 2>/dev/null | sed -n 's/^wpa_state=//p')
		[ "$state" = COMPLETED ] && return 0
		[ "$(date +%s)" -ge "$end" ] && return 1
		nap
	done
}

net_up() {
	[ -f "$CONF" ] && [ -n "$(conf ssid)" ] || return 3
	linked && return 4
	online && return 0

	w=0
	while [ ! -d "$NET/wlan0" ] && [ "$w" -lt 40 ]; do
		nap
		w=$((w + 1))
	done
	[ -d "$NET/wlan0" ] || return 1
	rfkill unblock wifi 2>/dev/null || true
	$IP link set wlan0 up 2>/dev/null || true

	stop_ours "$DHCP_PID" slotnet
	stop_ours "$SUP_PID" slotnet.conf
	if ! $WPA_CLI -p "$CTRL_DIR" -i wlan0 ping >/dev/null 2>&1; then
		/bin/mkdir -p "$CTRL_DIR"
		supplicant_conf
		linked && return 4
		$WPA_SUPPLICANT -B -i wlan0 -c "$SUP_CONF" -P "$SUP_PID" -Dnl80211 >/dev/null 2>&1 || return 1
	fi
	associated || return $?

	static="$(conf address)"
	if [ -n "$static" ]; then
		address "$static" "$(conf gateway)" "$(conf dns)"
	else
		client="$(dhcp_client)"
		[ -n "$client" ] || return 5
		{
			echo '#!/bin/sh'
			echo "exec /bin/sh \"$SELF\" dhcp \"\$@\""
		} > "$DHCP_HOOK"
		/bin/chmod 755 "$DHCP_HOOK"
		linked && return 4
		$client -i wlan0 -b -p "$DHCP_PID" -s "$DHCP_HOOK" -t 5 -T 3 >/dev/null 2>&1 || return 1
		w=0
		while [ ! -f "$LEASE" ] && [ "$w" -lt $((DHCP_WAIT_S * 4)) ]; do
			linked && return 4
			nap
			w=$((w + 1))
		done
		[ -f "$LEASE" ] || return 1
	fi

	if [ "$(conf ntp)" = on ] && command -v "$BUSYBOX" >/dev/null 2>&1; then
		("$BUSYBOX" ntpd -n -q -p pool.ntp.org >/dev/null 2>&1 &)
	fi
	return 0
}

net_down() {
	stop_ours "$DHCP_PID" slotnet
	stop_ours "$SUP_PID" slotnet.conf
	forget_address
	restore_dns
	/bin/rm -f "$SUP_CONF" "$DHCP_HOOK"
	return 0
}

case "${1:-}" in
up) net_up ;;
down) net_down ;;
status) online ;;
dhcp)
	shift
	dhcp_event "$@"
	;;
*)
	echo "usage: slotnet.sh [up|down|status]" >&2
	exit 2
	;;
esac

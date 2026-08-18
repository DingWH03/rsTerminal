#!/bin/sh
# rsterm-agent v0.2 — POSIX status reporter (compact tagged lines on stdout).
# Started via SSH exec (no PTY), usually fed on stdin (`sh -s`) or as a /tmp file.
#
# Wire (LF lines), delta tags:
#   H a=rsterm-agent v=0.2.0
#   S t=<unix_sec> [h=<host>] [m=<kib_t>,<kib_a>,<kib_u>] [d=<mount>,<kib_t>,<kib_a>] [c=<l1>,<l5>,<l15>] [w=<cwd>]
# Stable tags (h/d/w) are omitted when unchanged to save bandwidth.

# Re-exec under line buffering when available.
# Only when $0 is our uploaded script — `sh -s` leaves $0 as `sh`/`/bin/sh`.
if [ -z "${RSTERM_AGENT_BUFFERED:-}" ] && command -v stdbuf >/dev/null 2>&1; then
    case $0 in
        *rsterm-agent*)
            RSTERM_AGENT_BUFFERED=1 exec stdbuf -oL -eL sh "$0" "$@"
            ;;
    esac
fi

INTERVAL_MS=${RSTERM_INTERVAL_MS:-1000}
DISK_MOUNT=${RSTERM_DISK_MOUNT:-/}

PREV_H=
PREV_DT=
PREV_DA=
PREV_MOUNT=
PREV_W=
PREV_U=
SHELL_PID=${RSTERM_SHELL_PID:-}

to_u() {
    v=$1
    case $v in
        ''|*[!0-9]*) printf '0' ;;
        *) printf '%s' "$v" ;;
    esac
}

# Bytes → KiB (integer). Empty/garbage → 0.
to_kib() {
    b=$(to_u "$1")
    printf '%s' $((b / 1024))
}

emit_line() {
    printf '%s\n' "$1"
}

emit_line 'H a=rsterm-agent v=0.2.0'

read_mem_kib() {
    TOTAL=0
    AVAIL=0
    if [ -r /proc/meminfo ]; then
        TOTAL=$(awk '/^MemTotal:/ {printf "%.0f", $2*1024; exit}' /proc/meminfo 2>/dev/null || echo 0)
        AVAIL=$(awk '/^MemAvailable:/ {printf "%.0f", $2*1024; exit}' /proc/meminfo 2>/dev/null || echo 0)
        AVAIL=$(to_u "$AVAIL")
        if [ "$AVAIL" = "0" ]; then
            AVAIL=$(awk '/^MemFree:/ {printf "%.0f", $2*1024; exit}' /proc/meminfo 2>/dev/null || echo 0)
        fi
    fi
    TOTAL=$(to_u "$TOTAL")
    AVAIL=$(to_u "$AVAIL")
    USED=0
    if [ "$TOTAL" -gt 0 ] 2>/dev/null; then
        USED=$((TOTAL - AVAIL))
        if [ "$USED" -lt 0 ] 2>/dev/null; then
            USED=0
        fi
    fi
    printf '%s %s %s' "$(to_kib "$TOTAL")" "$(to_kib "$AVAIL")" "$(to_kib "$USED")"
}

read_cpu() {
    L1=0
    L5=0
    L15=0
    if [ -r /proc/loadavg ]; then
        L1=$(awk '{printf "%.2f", $1}' /proc/loadavg 2>/dev/null || echo 0)
        L5=$(awk '{printf "%.2f", $2}' /proc/loadavg 2>/dev/null || echo 0)
        L15=$(awk '{printf "%.2f", $3}' /proc/loadavg 2>/dev/null || echo 0)
    fi
    case $L1 in ''|*[!0-9.]* ) L1=0 ;; esac
    case $L5 in ''|*[!0-9.]* ) L5=0 ;; esac
    case $L15 in ''|*[!0-9.]* ) L15=0 ;; esac
    printf '%s %s %s' "$L1" "$L5" "$L15"
}

read_disk_kib() {
    MOUNT=${1:-/}
    LINE=$(df -Pk "$MOUNT" 2>/dev/null | awk 'NR==2 {print $2" "$4; exit}')
    TOTAL_K=$(printf '%s\n' "$LINE" | awk '{print $1}')
    AVAIL_K=$(printf '%s\n' "$LINE" | awk '{print $2}')
    # df -Pk already reports KiB.
    TOTAL_K=$(to_u "$TOTAL_K")
    AVAIL_K=$(to_u "$AVAIL_K")
    printf '%s %s %s' "$MOUNT" "$TOTAL_K" "$AVAIL_K"
}

read_cwd() {
    pid=$(resolve_shell_pid)
    if [ -n "$pid" ] && [ -r "/proc/${pid}/cwd" ]; then
        CWD=$(readlink -f "/proc/${pid}/cwd" 2>/dev/null || true)
        if [ -n "$CWD" ]; then
            printf '%s' "$CWD"
            return
        fi
        # Stale pid — rediscover next tick.
        SHELL_PID=
    fi
    printf ''
}

# Prefer RSTERM_SHELL_PID; else newest same-uid shell with a controlling tty.
# (SSH agent has no TTY; the interactive PTY shell does — started before us.)
resolve_shell_pid() {
    if [ -n "$SHELL_PID" ] && [ -r "/proc/${SHELL_PID}/cwd" ]; then
        printf '%s' "$SHELL_PID"
        return
    fi
    me=$(id -u 2>/dev/null || echo "")
    [ -n "$me" ] || return
    best=
    best_start=0
    for dir in /proc/[0-9]*; do
        [ -d "$dir" ] || continue
        pid=${dir#/proc/}
        [ -r "$dir/cwd" ] || continue
        [ -r "$dir/status" ] || continue
        [ -r "$dir/stat" ] || continue
        uid=$(awk '/^Uid:/{print $2; exit}' "$dir/status" 2>/dev/null || echo "")
        [ "$uid" = "$me" ] || continue
        # Field 7 = tty_nr; 0 means no controlling terminal.
        tty=$(awk '{print $7}' "$dir/stat" 2>/dev/null || echo 0)
        case $tty in
            ''|0) continue ;;
        esac
        # Comm is more reliable than cmdline for login shells (-bash, etc.).
        comm=$(tr -d '\0\n' <"$dir/comm" 2>/dev/null || true)
        case $comm in
            bash|zsh|fish|sh|dash|ksh|tcsh|csh) ;;
            *) continue ;;
        esac
        start=$(awk '{print $22}' "$dir/stat" 2>/dev/null || echo 0)
        start=$(to_u "$start")
        if [ "$start" -ge "$best_start" ] 2>/dev/null; then
            best=$pid
            best_start=$start
        fi
    done
    if [ -n "$best" ]; then
        SHELL_PID=$best
        printf '%s' "$best"
    fi
}

read_host() {
    if command -v hostname >/dev/null 2>&1; then
        hostname 2>/dev/null | tr -d '\n\r \t' || printf 'unknown'
    elif [ -r /proc/sys/kernel/hostname ]; then
        tr -d '\n\r \t' </proc/sys/kernel/hostname
    else
        printf 'unknown'
    fi
}

read_uptime() {
    if [ -r /proc/uptime ]; then
        awk '{printf "%.0f", $1; exit}' /proc/uptime 2>/dev/null || echo 0
    else
        printf '0'
    fi
}

emit_status() {
    TS=$(date +%s 2>/dev/null || echo 0)
    TS=$(to_u "$TS")
    HOST=$(read_host)
    CWD=$(read_cwd | tr -d '\n\r')
    UP=$(to_u "$(read_uptime)")
    set -- $(read_mem_kib)
    MT=$1 MA=$2 MU=$3
    set -- $(read_cpu)
    L1=$1 L5=$2 L15=$3
    set -- $(read_disk_kib "$DISK_MOUNT")
    DMOUNT=$1 DT=$2 DA=$3

    # Always send volatile tags; stable tags only on change.
    out="S t=${TS} m=${MT},${MA},${MU} c=${L1},${L5},${L15}"

    if [ "$HOST" != "$PREV_H" ]; then
        out="${out} h=${HOST}"
        PREV_H=$HOST
    fi
    if [ "$DMOUNT" != "$PREV_MOUNT" ] || [ "$DT" != "$PREV_DT" ]; then
        out="${out} d=${DMOUNT},${DT},${DA}"
        PREV_MOUNT=$DMOUNT
        PREV_DT=$DT
        PREV_DA=$DA
    elif [ -z "$PREV_DA" ]; then
        out="${out} d=${DMOUNT},${DT},${DA}"
        PREV_MOUNT=$DMOUNT
        PREV_DT=$DT
        PREV_DA=$DA
    else
        # Only re-emit disk when free space moves by ≥1 MiB.
        diff=$((DA - PREV_DA))
        if [ "$diff" -lt 0 ] 2>/dev/null; then
            diff=$((0 - diff))
        fi
        if [ "$diff" -ge 1024 ] 2>/dev/null; then
            out="${out} d=${DMOUNT},${DT},${DA}"
            PREV_DA=$DA
        fi
    fi
    # Uptime: first sample + about once per minute thereafter.
    if [ -z "$PREV_U" ] || [ $((UP - PREV_U)) -ge 60 ] 2>/dev/null || [ $((PREV_U - UP)) -ge 60 ] 2>/dev/null; then
        out="${out} u=${UP}"
        PREV_U=$UP
    fi
    if [ -n "$CWD" ] && [ "$CWD" != "$PREV_W" ]; then
        # w= must be last — may contain spaces.
        out="${out} w=${CWD}"
        PREV_W=$CWD
    fi
    emit_line "$out"
}

emit_status || emit_line 'E c=sample m=first_sample_failed'

while true; do
    SEC=$(( $(to_u "$INTERVAL_MS") / 1000 ))
    if [ "$SEC" -lt 1 ] 2>/dev/null; then
        SEC=1
    fi
    sleep "$SEC" || exit 0
    emit_status || true
done

#!/bin/sh
# Acceptance only: run as root in a disposable pinned OpenWrt VM.
set -eu
user=intent_witness
work=/tmp/intent-witness-permissions
acl=/usr/share/acl.d/intent-witness.json
sid=''
cleanup() {
    ubus call service delete '{"name":"intent-permission-check"}' >/dev/null 2>&1 || true
    if [ -n "$sid" ]; then
        ubus call session destroy "{\"ubus_rpc_session\":\"$sid\"}" >/dev/null 2>&1 || true
    fi
    rm -f "$acl"
    kill -HUP "$(pidof ubusd)" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT
[ "$(id -u)" = 0 ]
# Base images need not contain adduser or su. procd performs the uid switch.
if ! id "$user" >/dev/null 2>&1; then
    uid=30000
    while awk -F: -v uid="$uid" '$3==uid {found=1} END {exit !found}' /etc/passwd /etc/group; do
        uid=$((uid+1))
    done
    printf '%s:x:%s:\n' "$user" "$uid" >> /etc/group
    printf '%s:*:%s:%s:lab witness:/nonexistent:/bin/false\n' "$user" "$uid" "$uid" >> /etc/passwd
fi
mkdir -p "$work" /usr/share/acl.d
chmod 700 "$work"
chown "$user:$user" "$work"
# ubusd ACL schema is user/access/methods; rpcd ACL files use a different schema.
cat > "$acl" <<'ACL'
{"user":"intent_witness","access":{
 "session":{"methods":["access"]},
 "uci":{"methods":["get","changes"]},
 "network.interface":{"methods":["dump"]},
 "network.device":{"methods":["status"]},
 "network.wireless":{"methods":["status"]}
}}
ACL
chmod 644 "$acl"
chown 0:0 "$acl"
kill -HUP "$(pidof ubusd)"
sleep 1
[ "$(ubus list uci)" = uci ]
sid=$(ubus call session create '{"timeout":180}' | jsonfilter -e '@.ubus_rpc_session')
[ "${#sid}" = 32 ]
printf '%s' "$sid" > "$work/session"
chmod 400 "$work/session"
chown "$user:$user" "$work/session"
ubus call session grant "{\"ubus_rpc_session\":\"$sid\",\"scope\":\"ubus\",\"objects\":[[\"session\",\"access\"],[\"uci\",\"get\"],[\"uci\",\"changes\"],[\"network.interface\",\"dump\"],[\"network.device\",\"status\"],[\"network.wireless\",\"status\"]]}" >/dev/null
ubus call session grant "{\"ubus_rpc_session\":\"$sid\",\"scope\":\"uci\",\"objects\":[[\"network\",\"read\"]]}" >/dev/null
cat > "$work/check.sh" <<'CHECK'
#!/bin/sh
set -eu
work=/tmp/intent-witness-permissions
stage=identity
trap 'status=$?; printf "%s\n" "$stage" > "$work/step"; printf "%s\n" "$status" > "$work/exit"' EXIT
stage=identity_name
[ "$(id -un)" = intent_witness ]
stage=identity_uid
[ "$(id -u)" != 0 ]
stage=read_session
sid=$(cat "$work/session")
deny() {
    if "$@" > "$work/output" 2> "$work/error"; then return 1; fi
    # A malformed request, absent object, or missing executable is not denial.
    grep -Eq '^(Command failed: Permission denied|.*\(Permission denied\))$' "$work/error"
}
if [ "$(cat "$work/phase")" = revoked ]; then
    deny ubus call uci get "{\"config\":\"network\",\"ubus_rpc_session\":\"$sid\"}"
    exit 0
fi
stage=uci_read
ubus call uci get "{\"config\":\"network\",\"ubus_rpc_session\":\"$sid\"}" > "$work/read" 2> "$work/error"
stage=uci_values
[ "$(jsonfilter -i "$work/read" -e '@.values.lan.proto')" = static ]
stage=interface_read
ubus call network.interface dump '{}' > "$work/read"
[ "$(jsonfilter -i "$work/read" -e '@.interface[0].interface')" = lan ]
stage=device_read
ubus call network.device status '{}' > "$work/read"
[ "$(jsonfilter -i "$work/read" -e '@["br-lan"].present')" = true ]
stage=uci_write_denial
deny ubus call uci set '{"config":"network","section":"lan","values":{"intent_denied":"1"}}'
stage=interface_write_denial
deny ubus call network.interface up '{"interface":"lan"}'
stage=session_grant_denial
deny ubus call session grant "{\"ubus_rpc_session\":\"$sid\",\"scope\":\"uci\",\"objects\":[[\"network\",\"write\"]]}"
stage=session_write_permissions
for method in set add delete rename order commit apply confirm rollback revert reload_config; do
    allowed=$(ubus call session access "{\"ubus_rpc_session\":\"$sid\",\"scope\":\"ubus\",\"object\":\"uci\",\"function\":\"$method\"}" | jsonfilter -e '@.access')
    [ "$allowed" = false ]
done
CHECK
chmod 755 "$work/check.sh"
run_check() {
    printf '%s' "$1" > "$work/phase"
    chmod 644 "$work/phase"
    rm -f "$work/exit"
    ubus call service set '{"name":"intent-permission-check","instances":{"check":{"command":["/bin/sh","/tmp/intent-witness-permissions/check.sh"],"user":"intent_witness","group":"intent_witness"}}}' >/dev/null
    n=0
    while [ ! -f "$work/exit" ] && [ "$n" -lt 20 ]; do n=$((n+1)); sleep 1; done
    ubus call service delete '{"name":"intent-permission-check"}' >/dev/null
    if [ ! -f "$work/exit" ] || [ "$(cat "$work/exit")" != 0 ]; then
        printf "permission check failed: %s/%s\n" "$1" "$(cat "$work/step" 2>/dev/null || echo no_child_result)" >&2
        sed 's/[0-9a-f]\{16,\}/[redacted]/g' "$work/error" 2>/dev/null || true
        return 1
    fi
}
run_check valid
ubus call session destroy "{\"ubus_rpc_session\":\"$sid\"}" >/dev/null
sid=''
run_check revoked
printf '\n%s%s\n' INTENT_PERMISSION_ PASS

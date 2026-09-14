# Secrets and wireless templates

The compiler accepts **references to credentials**, never literal Wi-Fi passwords.
It does not read a secret store, environment variable, private file, or router.
The backup fixtures contain redaction markers; the example contains identifiers.

## Declare an opaque reference

```text
access-point default_radio1 {
  radio radio1
  interface lan
  mode ap
  ssid "_iot"
  security psk2
  credential secret://core-router/wifi/iot
}
```

`secret://core-router/wifi/iot` is a logical identifier selected by the operator.
It is not a URL, filename, shell expression, or an instruction to fetch anything.
An external consumer decides how that identifier maps to an existing secret.
Reusing an identifier means reusing the same credential; different identifiers
can also be mapped to the same externally managed value.

Wi-Fi identifiers are at most 255 ASCII characters, beginning with `secret://`,
followed by nonempty slash-separated segments containing letters, digits,
underscores, hyphens, or dots. Entire `.` and `..` segments are forbidden.
Whitespace, query strings, interpolation, and executable fragments are rejected.
Malformed credential diagnostics do not echo the supplied value.

`psk2` and `sae` require a credential reference, including for disabled secured
APs. An open AP (`security none`) must not declare a credential. SSIDs and other
nonsecret fields remain ordinary strings. A Wi-Fi credential is a distinct
`SecretRef WiFiCredential` type; password hashes and other secret kinds cannot
be substituted for it.

## Compilation results

```sh
./netc compile examples/core-router.net --target gateway --format json
```

When any generated wireless section needs credentials, the result contains:

- `/etc/config/wireless.template`, instead of `/etc/config/wireless`;
- `secret-bindings.json`, describing every placeholder;
- `requiresSecretBinding: true` and `readiness: "requires-secret-binding"`;
- source-map entries for both the template and its binding manifest.

The wireless template begins with a notice that it requires binding. Its UCI
values contain deterministic placeholders, for example:

```text
config wifi-iface 'default_radio1'
    option device 'radio1'
    option network 'lan'
    option mode 'ap'
    option ssid '_iot'
    option encryption 'psk2'
    option key '__NETC_SECRET_default_radio1_key__'
```

The corresponding manifest entry is:

```json
{
  "manifestVersion": 1,
  "status": "requires-secret-binding",
  "bindings": [
    {
      "reference": "secret://core-router/wifi/iot",
      "kind": "wifi-credential",
      "security": "psk2",
      "templateArtifact": "/etc/config/wireless.template",
      "installationPath": "/etc/config/wireless",
      "section": "default_radio1",
      "option": "key",
      "placeholder": "__NETC_SECRET_default_radio1_key__"
    }
  ]
}
```

Placeholder identity derives from the UCI section and option, not the secret
value. Section identifiers are checked and unique within each package. Multiple
APs may use one credential reference but have separate binding entries.

If no wireless credentials are required, the compiler emits ordinary wireless
configuration with no binding manifest. This means only that no binding step is
needed; configuration remains **Intended**, not Applied or Observed.

**OpenWrt cannot resolve `secret://` references. Do not install the template.**
A placeholder can be interpreted as an actual password by device software;
the artifact path, readiness metadata, and manifest distinguish the template
from completed configuration.

## Contract for an external binding consumer

Retrieval, materialization, and deployment are deliberately not implemented.
A separate consumer must:

1. Check the manifest version and the expected artifacts. Restrict installation
   paths to its approved destination set; never execute artifact contents.
2. Resolve each opaque reference through an explicitly configured private
   source. The compiler's identifier syntax does not prescribe a storage system.
3. Validate each resolved value against its credential kind, security mode, and
   the actual target firmware's password requirements. WPA2 passphrases/PSKs and
   SAE passwords require their respective validation; do not treat the modes
   as interchangeable. Reject unsupported controls or line separators.
4. Parse the UCI template and locate the exact named section and option from the
   manifest. Require that its current value equals the expected placeholder.
   Bind the structured value; do not perform global text replacement, `eval`,
   shell interpolation, or replacement inside unrelated settings such as SSIDs.
5. Serialize with UCI quoting. Enclose values in single quotes and encode an
   embedded apostrophe by closing the quote, emitting `\'`, then reopening it.
   For a synthetic illustration, `example'phrase` becomes `'example'\''phrase'`.
   Backslashes and double quotes inside single-quoted UCI values remain data.
6. Fail without producing installation-ready output if any reference is missing,
   a value is invalid, a section is ambiguous, a placeholder differs, or an
   expected binding remains unresolved. Complete all bindings before publishing
   the final artifact under its installation path.
7. Treat completed configuration as secret material: use restrictive file
   permissions, exclude it from source control and public build artifacts, and
   keep credential values out of logs, error messages, and review diffs.

Reference identifiers may reveal inventory names; choose them accordingly.
Changing an external secret does not change deterministic compiler output.
Rotation requires rerunning the external binding step and separately applying
its result. No secret rotation or device operation happens during compilation.

## Assurance and tests

The semantic export contains `credentialRef`, never credential values. The
formatter preserves those references. Source maps identify declarations and
bindings without copying source lines. Literal values in the `credential` field
are rejected; the compiler does not attempt to discover secrets hidden in
ordinary nonsecret strings.

The test suite verifies reference validation and redacted errors, required
credentials, secret-kind separation, deterministic placeholders, manifest/template
agreement, source maps, secret-free output, and UCI quoting with synthetic data.
It does not certify an external consumer or claim that any credential was
retrieved, validated against a running device, or installed.

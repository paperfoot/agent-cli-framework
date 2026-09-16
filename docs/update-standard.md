# Update standard

One `update` command, with behavior determined by the installation owner.
Package managers own their binaries. Standalone installations may self-replace
only after implementing the verification and replacement policy below.

## The reference implementation

The greeter returns installation instructions. It does not query releases,
download an asset, run a package manager, or replace itself. Both `update` and
`update --check` report `latest_version: null` and `status: "not_checked"` unless
updates are disabled. This is an explicit scaffold boundary, not a completed
release check. Replace the repository/package placeholders when copying it.

A distributed tool must implement its actual release lookup and test the
appropriate upgrade path. Until then, retain honest instructions-only behavior.

## Command contract

`update --check` never changes the installation, shell profile, or package-manager
state. It emits the normal envelope and exits 0 when the check completes, including
when an update exists. A failed lookup exits 1; invalid configuration exits 2.
It must not create an operational lock simply to return local instructions.

`update` either applies through the correct owner or returns a tested instruction.
It must not overwrite a package-managed binary with a raw downloaded asset.

The success payload always includes these fields:

```json
{
  "current_version": "1.2.3",
  "latest_version": "1.2.4",
  "status": "update_available",
  "install_source": "homebrew",
  "update_mode": "package_manager",
  "upgrade_command": "brew upgrade your-cli",
  "release_url": "https://github.com/owner/repo/releases/tag/v1.2.4",
  "requires_skill_reinstall": true
}
```

| Field | Contract |
| --- | --- |
| `current_version` | Installed binary version |
| `latest_version` | Verified available version, or null when not checked |
| `status` | `not_checked`, `up_to_date`, `update_available`, `updated`, `disabled`, `managed_install`, or `unsupported_platform` |
| `install_source` | `standalone`, `homebrew`, `cargo`, `cargo_binstall`, `npm`, `bun`, `uv_tool`, `pipx`, `winget`, `scoop`, `apt`, `managed`, or `unknown` |
| `update_mode` | `self_replace`, `package_manager`, `instructions_only`, or `disabled` |
| `upgrade_command` | A literal command for the configured package, or null; never prose disguised as a command |
| `release_url` | Relevant release/instructions URL, or null |
| `requires_skill_reinstall` | Whether an applied or available update requires refreshing the installed skill; false when unknown |

These are command-payload statuses, separate from the envelope's status.
The manifest must describe the installed implementation, including whether
`--check` actually consults a release source. Never imply a check succeeded by
copying the installed version into `latest_version`.

## Determine ownership

Prefer explicit configuration, then build/installer metadata, then reliable
executable-path or package-manager ownership evidence. A path under a development
`target/` directory does not prove standalone installation. Uncertain ownership
means `unknown` and `instructions_only`.

Use the configured package/formula identifier, preserving the installation's
registry, tap, and pin policy. Validate or correctly quote dynamic values in any
returned shell command. Do not switch a package-manager install to another channel.
Managed installations follow the owner's rollout process. Disabled updates return
`disabled`, exit 0, and no executable upgrade suggestion.

For example, Homebrew uses `brew upgrade <formula>` and Cargo uses
`cargo install --locked --force <crate>`. Use cargo-binstall only when that channel
and the required artifacts are supported. Other managers retain their own upgrade
semantics. The [uv installation policy](https://docs.astral.sh/uv/getting-started/installation/#upgrading-uv)
is an example of this ownership boundary; [Cargo documents its install options](https://doc.rust-lang.org/cargo/commands/cargo-install.html).

## Standalone replacement

Before enabling `self_replace`, implement and test all of the following:

1. Select the exact supported OS, architecture, libc, and binary asset. Respect
   the configured stable/prerelease policy.
2. Download over HTTPS with finite size/time limits. Verify the artifact's SHA256
   against trusted release metadata before extraction or execution. Verify any
   configured signature or attestation policy as well.
3. Extract safely into a temporary directory on the target filesystem. Reject
   path traversal and unexpected executable names.
4. Validate the staged binary with `--version` and ensure it matches the requested
   release. Keep its stdout inside the parent command's structured result.
5. Hold an appropriate update lock; atomically replace where supported. Preserve
   the installed binary when download, integrity, validation, or replacement fails.
6. Report the installed version and whether `skill install` is needed. Test
   interruption and concurrent attempts without corrupting either installation.

A downloader crate alone does not establish these guarantees. The example omits
one until the tool's release pipeline and verification policy are implemented.

## Release pipeline

Build release artifacts in CI, not on a developer laptop. Use cargo-dist or an
equivalent reproducible pipeline for archives, checksums, installers, and supported
package-manager metadata. Publish only platforms that are built and tested.
Provide provenance/attestations when required by the deployment policy.

Smoke-test each supported install channel and its upgrade path, then run
`agent-info`, `update --check --json`, and the framework conformance checks against
the installed binary. A successful compile is not an installation test.

The example's active settings are in `UpdateConfig`: `enabled`, `install_source`,
`owner`, `repo`, `crate_name`, `formula`, and `tap`. Add new checksum, signature, or
prerelease settings only with code that enforces them; avoid decorative config.

## Required checks

- Check mode emits one valid envelope without mutation or extra stdout.
- Disabled, unknown, and package-managed sources never self-replace.
- Unknown release state remains null/`not_checked`.
- Actual checks distinguish current, available, failed, and unsupported outcomes.
- Returned commands name the correct package and parse safely as shell commands.
- Real downloaders reject corrupt/wrong-platform assets and preserve the old binary.
- A second updater cannot invalidate the active owner's lock.
- Manifest options and effects match the implemented update paths.

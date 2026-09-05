# Filesystem handles and publication boundaries

The current server filesystem primitives are being hardened and adopted in P8. This document describes the implemented Unix/Linux server boundary, not completion of every server consumer's acceptance gates. Windows/macOS Agent filesystem contracts belong exclusively to sarmg-foundation-agent.

## Unix private state

`PrivateDirectory::open_existing` walks the same no-follow descriptors and checks
the same owner and permissions without creating directories, changing modes or
syncing files. Read-only server diagnostics use this entry point.

`create_child` creates or validates a typed direct child relative to the held
parent descriptor on Unix, so replacing the original state pathname cannot
redirect initialization of server-owned child state.

`PrivateDirectory::create` requires an absolute path. It walks existing ancestors through no-follow directory descriptors and creates only the final directory with mode 0700. The final directory must be owned by the effective user with exact 0700 permissions. An existing directory is validated, never chmodded; rejection cannot change a symlink target's permissions. Parent and new-directory sync complete creation.

`AtomicFile::replace` and `AdvisoryLock::acquire` accept a validated, single-component relative name under the held private-directory descriptor. They do not resolve mutations through the original absolute pathname. A renamed or replaced external pathname therefore cannot redirect the operation. Atomic files and lock files are created as 0600; special files, symlinks, and hardlinked targets are rejected. The stable lock inode remains after release.

Atomic replacement writes and syncs the temporary file, renames within the held directory, checks the published inode, and syncs the directory. Failure after publication is not proof of rollback; parent-sync failure is explicitly `PublishedDurabilityUnknown`.

`EntryName` represents exactly one canonical filename. `PrivateDirectory::files`, `read_bounded` and `remove_file` use these typed names and the held descriptor on Unix, not reconstructed absolute paths. `AtomicFile::create` provides no-clobber creation; failed collision preserves the occupant. Platform temporaries have one exact random namespace, exposed through `AtomicFile::is_temporary_name` for cleanup under exclusive process ownership. Client filesystem implementations are separately owned by sarmg-foundation-agent.

`NoClobberPublish::publish` accepts a private directory and two typed single-component names, not arbitrary source/destination strings. It only publishes a single-linked regular file within that directory. Linux uses `RENAME_NOREPLACE`, with no fallback on unsupported filesystems. The Unix implementation for other operating systems uses link, parent sync, unlink, parent sync. Publication or sync failures must not be interpreted as permission to blindly replay a mutation.

Inventory opens directories descriptor-relatively and refuses symlinks, special files and multiply-linked files. It checks pre/post-open identity and enforces entry, byte and 128-directory-depth budgets; depth-first traversal bounds simultaneous directory handles independent of directory width. File/parent synchronization on Unix also opens through no-follow descriptors and refuses non-regular or multiply-linked files.

These private-state primitives require exclusive application ownership of the directory. They do not claim to defend against a malicious process running as the same user and concurrently modifying the private namespace. Advisory locks only coordinate cooperating participants.

## Linux rooted filesystems

`OpenAt2Root` verifies the initial directory and probes openat2 before serving work. `MountPolicy` makes cross-mount access an explicit technical capability. There is no openat fallback when the required Linux primitive is unavailable.

`open_file` only returns single-linked regular files and refuses symlinks in all components; NONBLOCK prevents FIFO type probes from hanging. `FileIdentity` and `SingleLinkRequirement` describe opened objects. The Linux directory `AdvisoryLock` holds the anchored root itself instead of a replaceable separate lock pathname.

Products may retain business-specific symlink, upload metadata, tree mutation and crash-recovery semantics while their generic helpers are progressively replaced. Dufs currently consumes the shared root initialization/probe, root lock and file identity. Its remaining rooted operations have not yet all moved upstream. Server consumers must not use the diagnostic `PrivateDirectory::path`/`resolve` values as a substitute for held-handle mutations.

## Remaining acceptance

Server-side raw-path staging operations, cross-directory publication, bounded inventories at every consumer, and Upgrade adoption still require implementation and acceptance. Passing the Linux library tests is not P8 completion. Agent Spool and native client acceptance are tracked only in sarmg-foundation-agent, not governed by this server specification.

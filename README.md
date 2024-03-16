# bwfs

_A userspace filesystem (FUSE) for bitwarden_

**Currently very proof of concept**

## Why?

You can run a big electron application to access your secrets, or you can use a CLI, both with special ways to get the content you want.

Or, now you can use the filesystem and your normal tools, everything is a file on UNIX after all!

## Usage

**This is currently very early stage so don't expect stability or it even working reliably**

As a precondition, you should have the official bitwarden CLI client installed and available on your `PATH` as `bw`.
You will have to have done an initial login so that the client knows your basic account info such as username.

Then, to run `bwfs` from the root of this project:

```
cargo run -- serve <mountpoint>
```

Then in another terminal, run:

```
# see the current status of bwfs
cargo run -- status

# unlock the filesystem and refresh its contents
cargo run -- unlock
# prompts for password

# when you're done, you can lock it manually
cargo run -- lock
# this removes the contents from being accessible through the mountpoint
```

### `allow_other` issues

If you have problems with executing it such as
> fusermount3: option allow_other only allowed if 'user_allow_other' is set in /etc/fuse.conf
you can run it without auto unmounting:

```
cargo run -- --no-auto-unmount <mountpoint>
```

When you're done, you'll need to unmount the directory manually:

```
umount <mountpoint>
```

## Security

- [x] Secrets are never persisted to disk directly
- [ ] Secrets may currently be persisted to SWAP memory

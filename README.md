# bwfs

_A userspace filesystem (FUSE) for bitwarden_

**Currently very proof of concept**

## Why?

You can run a big electron application to access your secrets, or you can use a CLI, both with special ways to get the content you want.

Or, now you can use the filesystem and your normal tools, everything is a file on UNIX after all!

## Usage

**This is currently very early stage so don't expect stability or it even working reliably**

As a precondition, you should have the official bitwarden client installed and available on your `PATH` as `bw`.
You will have to have done an initial login so that the client knows your basic account info such as username.

Then, to run bwfs from the root of this project:

```
cargo run -- <mountpoint>
# asks for password if locked
# listing the secrets takes a while in the bw tool
# after a bit you can `ls <mountpoint>` to see your secrets
```

## Security

- [x] Secrets are never persisted to disk directly
- [ ] Secrets may currently be persisted to SWAP memory


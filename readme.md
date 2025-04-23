# janitors

`janitors` is a helpful little program which keeps your directories nice and clean. All you need is a single configuration, which defines the directories to watch and rules for what to do with new files which are created inside these directories.

## Configuration file

`janitors` looks for the following configuration file in the given order:

- `~/.config/janitors/config.toml`
- `~/.janitors.toml`
- `/etc/janitors/janitors.toml`

The configuration file is in the `toml` format. Here is a simple example with more detailed explanation below:

```toml
[[watch]]
path = "~/Downloads/"
recursive_mode = "non-recursive"
bucket_names = ["documents", "scripts", "archives", "unsorted"]

[[bucket]]
name = "documents"
destination = "~/Downloads/documents/"
extension_filters = ["txt", "md", "pdf", "doc", "docx", "ppt", "pptx"]
name_filters = []
priority = 10
action = "move"
override_action = "skip"

[[bucket]]
name = "scripts"
destination = "~/Downloads/scripts/"
extension_filters = ["sh", "py", "js"]
name_filters = []
priority = 10
action = "copy"
override_action = "overwrite"

[[bucket]]
name = "archives"
destination = "~/archives"
extension_filters = ["zip", "rar"]
name_filters = [".*\\.tar.*"]
priority = 10
action = "move"
override_action = "rename"

[[bucket]]
name = "unsorted"
destination = "~/Downloads/unsorted/"]
extension_filters = []
name_filters = [".*"]
priority = 0
action = "move"
override_action = "rename"
```

The above configuration defines a single watch path: `~/Downloads`. The path is scanned for any new files. If a new file is created in that directory, the new file is attempted to be "placed" into one of the _buckets_ from `bucket_names`.

The `recursive_mode` field in a watch path definition defines whether `janitors` should also look at subdirectories or not. The possible values are:

- `"non-recursive"` - do not look into subdirectories.
- `"recursive"` - look for new files in subdirectories.

A bucket filters files which will be "placed" inside of it based on the filename extensions of the file(`extension_filters`) or whether the filename matches a regular expression(`name_filters`).

A bucket has a `priority`, if a file fits into multiple buckets the one with the highest priority is chosen. The priority is a 32-bit unsigned integer, where 0 is the lowest priority.

Each bucket has an associated action. The possible actions are:

- `"move"` - move the file into the bucket.
- `"copy"` - copy the file into the bucket, leaving a copy of the file in the watch directory.
- `"delete"` - delete the file.

Each bucket also has an `override_action` associated with it. This specifies the behavior of `janitors` when a file with the same name already exists in the bucket destination. The override actions are:

- `"skip"` - this is the default behavior, the file is skipped.
- `"overwrite"` - the file in the destination is overwritten with the contents the new file.
- `"rename"` - the new file is renamed by appending a `.[num]` extensions to the name. The `num` is the next unused natural number.

## Running janitors

`janitors` can run persistently as a daemon or it can run a single time which is called a "one-shot" mode. The two modes differ, because the "daemon" mode only watches for new files being created while the "one-shot" mode scans all existing watch paths and attempts to apply bucket rules on all existing files.

### Setting up janitors as a daemon

If you are using `systemd` you can create a custom `systemd` file to run `janitors` on startup:

```config
[Unit]
Description=janitors cleaning daemon

[Service]
ExecStart=/path/to/janitors/binary

[Install]
WantedBy=default.target
```

Name the file `janitors.service` and place it into `~/.config/systemd/user/`. Run `systemctl --user enable janitors.service` to enable the service to start on startup. Than you can manually start it by `systemctl --user start janitors.service`. To check the status and logs, run `systemctl --user status janitors.service`.

### Running in one-shot mode

To run `janitors` in one-shot mode, run `janitors --one-shot`.

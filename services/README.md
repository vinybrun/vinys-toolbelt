# services

Long-running daemons and their unit files.

Each service gets a directory containing the executable plus a `*.service` systemd
unit. Install as a user unit:

```
systemctl --link --user enable /code/vinys-toolbelt/services/<name>/<name>.service
systemctl --user start <name>
journalctl --user -u <name> -f
```

`--link` keeps the unit in the repo instead of copying it into `~/.config/systemd`,
so edits here take effect after `systemctl --user daemon-reload`.

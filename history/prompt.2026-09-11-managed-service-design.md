# Managed history service

Issue: codeloops-dwy

`make start` must return to the shell after a healthy background service starts.
Use systemd user services on Linux and launchd LaunchAgents on macOS for lifetime,
login startup, crash recovery, and logs. Keep `make run` for foreground development.
Detached shell processes and PID files would duplicate those responsibilities.

The application owns service definitions and a write-ahead ownership record inside
the install prefix. Derive the service name from the profile and canonical prefix
to isolate installations. Commands use saved absolute paths, never shell startup
files or inherited archive settings. Repeating start reloads the installed binary.
Stop disables login startup; uninstall stops and removes owned service state before
removing the executable, preserving history and unrelated configuration.

Expose `service start`, `stop`, `status`, and `logs` through the installed CLI and
matching Make targets. Report readiness only after authenticated HTTP health works.
An occupied unmanaged port gets an explicit instruction to stop the older foreground
copy once. Do not kill an unidentified listener.

Implementation sequence: add the lifecycle module and installation seam; wire CLI
and Make; exercise ownership, manager commands, readiness/failure, and terminal
independence in isolated fixtures; verify a unique service with the actual Linux
user manager; align documentation; run make check/e2e and UBS; publish a PR.

Native macOS behavior requires macOS verification; command/definition tests on
Linux must not be reported as a live launchd run.

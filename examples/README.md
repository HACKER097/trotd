# trotd Integration Examples

This directory contains examples of how to integrate `trotd` into various workflows.

## MOTD (Message of the Day)

### Automatic Setup Script

Run the automated setup script:

```bash
sudo bash examples/motd-setup.sh
```

This will configure your system to show trending repos on login.

### Manual Shell Integration

For bash, add to `~/.bashrc`:

```bash
if command -v trotd &> /dev/null; then
    trotd 2>/dev/null || true
fi
```

For zsh, add to `~/.zshrc`:

```zsh
if command -v trotd &> /dev/null; then
    trotd 2>/dev/null || true
fi
```

For fish, add to `~/.config/fish/config.fish`:

```fish
if command -v trotd &> /dev/null
    trotd 2>/dev/null; or true
end
```

### Advanced: Show Once Per Day

```bash
TROTD_CACHE="$HOME/.cache/trotd/.shown_today"
TODAY=$(date +%Y-%m-%d)

if [ ! -f "$TROTD_CACHE" ] || [ "$(cat $TROTD_CACHE)" != "$TODAY" ]; then
    if command -v trotd &> /dev/null; then
        trotd && echo "$TODAY" > "$TROTD_CACHE"
    fi
fi
```

## Systemd Timer (Auto-Refresh)

Keep your cache always fresh with a systemd user timer:

```bash
bash examples/systemd-timer-refresh.sh
```

This creates a timer that refreshes the cache every hour in the background.

## Shell Completions

Generate shell completions for better UX:

### Bash

```bash
trotd completions bash > /etc/bash_completion.d/trotd
```

Or for user-only:

```bash
mkdir -p ~/.local/share/bash-completion/completions
trotd completions bash > ~/.local/share/bash-completion/completions/trotd
```

### Fish

```bash
trotd completions fish > ~/.config/fish/completions/trotd.fish
```

### Zsh

```bash
mkdir -p ~/.zsh/completions
trotd completions zsh > ~/.zsh/completions/_trotd
```

Then add to `~/.zshrc`:

```zsh
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

## Tips & Tricks

### Filter by Language

Show only Rust and Go repositories:

```bash
trotd --lang rust,go
```

### Filter by Star Count

Show only popular repos (100+ stars):

```bash
trotd --min-stars 100
```

### Exclude Topics (GitHub Only)

Exclude certain topics from GitHub results:

```bash
trotd --exclude-topics awesome,awesome-list
```

### Combine Filters

```bash
trotd --lang rust --min-stars 50 --exclude-topics web
```

### Use in Scripts

```bash
#!/bin/bash
# Daily digest email

{
    echo "Subject: Daily Trending Repos"
    echo ""
    trotd
} | sendmail user@example.com
```

### JSON Output for Custom Processing

```bash
# Get just the repo names
trotd --json | jq -r '.[].name'

# Get repos with their star counts
trotd --json | jq -r '.[] | "\(.name): \(.stars_total) stars"'

# Filter JSON output
trotd --json | jq '.[] | select(.stars_total > 100)'
```

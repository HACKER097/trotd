#!/bin/bash
# Example script showing how to integrate trotd into your shell profile
# This will show trending repos every time you open a new terminal

# For bash, add to ~/.bashrc:
# ----------------------------------------
# if command -v trotd &> /dev/null; then
#     trotd 2>/dev/null || true
# fi

# For zsh, add to ~/.zshrc:
# ----------------------------------------
# if command -v trotd &> /dev/null; then
#     trotd 2>/dev/null || true
# fi

# For fish, add to ~/.config/fish/config.fish:
# ----------------------------------------
# if command -v trotd &> /dev/null
#     trotd 2>/dev/null; or true
# end

# Advanced: Only show once per day
# ----------------------------------------
# TROTD_CACHE="$HOME/.cache/trotd/.shown_today"
# TODAY=$(date +%Y-%m-%d)
#
# if [ ! -f "$TROTD_CACHE" ] || [ "$(cat $TROTD_CACHE)" != "$TODAY" ]; then
#     if command -v trotd &> /dev/null; then
#         trotd && echo "$TODAY" > "$TROTD_CACHE"
#     fi
# fi

# Advanced: Show only for interactive shells and not in tmux/screen
# ----------------------------------------
# if [[ $- == *i* ]] && [ -z "$TMUX" ] && [ -z "$STY" ]; then
#     if command -v trotd &> /dev/null; then
#         trotd 2>/dev/null || true
#     fi
# fi

echo "Copy the appropriate snippet above to your shell configuration file!"

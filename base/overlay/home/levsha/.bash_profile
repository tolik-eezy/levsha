# Levsha OS — launch Wayland session on tty1
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    if command -v cage &>/dev/null; then
        exec levsha-session
    else
        echo ""
        echo "  Levsha OS — cage compositor not found."
        echo "  Run: sudo dnf install -y cage"
        echo ""
    fi
fi

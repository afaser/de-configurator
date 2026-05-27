# de-configurator 🚀

**de-configurator** is a lightweight, blazing-fast background daemon written in Rust that manages your entire Linux desktop environment configuration (specifically tailored for tiling window managers like `bspwm` or `hyprland`).

If you are tired of maintaining dozens of scattered startup scripts, setting up keybindings in one utility (`sxhkd`), window placement rules in another, and VPN profiles in a third — this project is for you. It unifies them all into a single reactive system managed by one human-readable configuration file written in `KDL`.

---

## 💡 What does it do? (In plain terms)

Imagine your desktop environment having a single, unified "brain". The daemon runs in the background and:
1. **Listens to Keyboard**: Captures global hotkeys globally (including multimedia keys like volume and player control).
2. **Tracks Windows**: Detects when you switch workspaces, create new windows, or change window focus.
3. **Automates Startup Services**: Starts and monitors background utilities (compositor like `picom`, notification daemon like `dunst`, etc.).
4. **Manages VPN**: Toggles and establishes VPN connections via hotkeys, CLI actions, or reactive triggers.
5. **Places Windows Across Monitors**: Allows you to send applications (e.g., Brave, Telegram, Discord) to specific monitors on designated workspaces with a single hotkey, automatically handling rules and focus.

---

## 🛠 Key Features

- **All-in-One Configuration (`config.kdl`)**: No more bash-spaghetti. Hotkeys, autostart, workspaces, and VPN profiles are defined in a single structured file.
- **Smart Application Workspaces**: Move application windows (by window class) to specific target monitors on demand, with automatic window redirection rules (spawn rules) for newly opened windows.
- **Reactive Triggers**: Automatically execute commands or actions when system events occur (e.g., "when desktop X gains focus, run script Y").
- **IPC Interface**: Creates a local UNIX domain socket. You can control the daemon from the command line or external scripts using the `de-action` CLI utility.
- **Written in Rust**: Exceptionally low memory and CPU footprint, zero memory leaks (leveraging the `mimalloc` allocator), and runs instantly.

---

## 🚀 Quick Start

### 1. Requirements
- Linux with X11 session running (for global hotkeys).
- Installed window manager (currently fully supports `bspwm`, with architectural groundwork for `hyprland`).
- `playerctl` (for player media controls), `xprop` (for window class queries).

### 2. Build
Build the project using Cargo:
```bash
cargo build --release
```
The compiled binary will be located at `target/release/de-configurator`.

### 3. Example Configuration
Create your configuration file. The daemon looks for it at `~/.config/de-configurator/config.kdl` (or in the current working directory during development).

Example of a simple `config.kdl`:
```kdl
// Enable workspaces and bind window classes
feature "workspaces" enabled=true {
    workspace "browser" class="Brave-browser"
    workspace "telegram" class="TelegramDesktop"
}

// Manage background services
feature "daemons" enabled=true {
    daemon "picom" exec="picom" autostart=true
    daemon "dunst" exec="dunst" autostart=true
}

// Define hotkeys
feature "keybinds" enabled=true {
    // Launch terminal
    bind "alt+c" {
        run "alacritty"
    }

    // Move Telegram workspace to HDMI-0 monitor
    bind "alt+a" {
        workspace "telegram" "HDMI-0"
    }

    // Media keys (automatically normalized under the hood)
    bind "mediaplaypause" {
        run "playerctl" "play-pause"
    }
}
```

### 4. Running the Daemon
Start the daemon in the background (e.g., from your X11 startup profile):
```bash
de-configurator &
```

---

## 🎮 Controlling via CLI (`de-action`)

You can send commands directly to the daemon via the socket `/tmp/de-configurator.sock` using the `de-action` CLI.

Example commands:
```bash
# Switch VPN profile to "warp"
de-action vpn warp

# Start, stop, or restart picom daemon
de-action daemon picom toggle

# Activate browser workspace on VGA-0 monitor
de-action workspace browser VGA-0
```
Using `de-action`, you can easily trigger complex desktop environment modifications from external tools and shell scripts.

---

## ⚙️ Detailed Configuration Guide (config.kdl)

The configuration file uses the clean, human-readable [KDL](https://kdl.land/) format. Below are all supported features and their configuration syntax.

### 1. VPN Connections (`feature "vpn"`)
Defines different VPN profiles.
* `state "state_id"` — unique profile identifier (e.g., `"warp"`, `"finland"`, `"off"`).
* `display-name` — user-friendly name for notifications.
* `interface` — name of the VPN network interface.
* `up-cmd` / `down-cmd` — commands (with arguments) to establish and tear down the connection.

Example:
```kdl
feature "vpn" enabled=true {
    state "off" {
        display-name "VPN Off"
    }
    state "warp" {
        display-name "Cloudflare Warp"
        interface "wg-warp"
        up-cmd "wg-quick" "up" "warp"
        down-cmd "wg-quick" "down" "warp"
    }
}
```

### 2. Background Daemons (`feature "daemons"`)
Controls background system services.
* `daemon "name"` — identifier of the daemon.
* `autostart=true` — whether to run it automatically on startup.
* `display-name` — user-friendly name for notifications.
* `start-cmd` — executable command with all arguments.

Example:
```kdl
feature "daemons" enabled=true {
    daemon "picom" autostart=true {
        display-name "Picom Compositor"
        start-cmd "picom" "-b"
    }
}
```

### 3. Actions (`feature "actions"`)
Allows grouping command sequences into reusable macro-like actions.
* `action "name" export=true` — actions with `export=true` can be directly executed via `de-action` CLI.
* Inside an action block, you can use:
  * `vpn "state_id"` — switch VPN state.
  * `daemon "daemon_name" "on|off|toggle"` — control daemons.
  * `workspace "workspace_name" ["monitor_name"]` — navigate workspaces.

Example:
```kdl
feature "actions" enabled=true {
    action "game-mode" export=true {
        vpn "warp"
        daemon "picom" "off" // disable compositor for performance
    }
}
```

### 4. Reactive Triggers (`feature "triggers"`)
Binds automatic execution to various system events.
* `trigger "name" type="wm"` — trigger based on Window Manager events:
  * `event "desktop_focus"` — fires on desktop switch.
  * `event "node_focus"` — fires on window focus change.
* `trigger "name" type="interval"` — trigger on a timer:
  * `interval "1m"` (1 minute, supports `s` for seconds, `h` for hours).
* `run "command" "args"...` — shell command to execute.

Example:
```kdl
feature "triggers" enabled=true {
    trigger "periodic-check" type="interval" {
        interval "5m"
        run "notify-send" "DE" "System check run"
    }
}
```

### 5. Application Workspaces (`feature "workspaces"`)
Binds window classes (retrieve class name using `xprop`) to logical workspaces.
* `workspace "workspace_name" class="WindowClass"` — e.g., `Brave-browser`, `AyuGramDesktop`, `discord`.

Example:
```kdl
feature "workspaces" enabled=true {
    workspace "browser" class="Brave-browser"
    workspace "telegram" class="AyuGramDesktop"
}
```

### 6. Hotkeys (`feature "keybinds"`)
Binds key shortcuts to execution actions.
* Supports modifier keys: `ctrl`, `shift`, `alt`, `super`.
* Automatically normalizes media keys: `mediaplaypause`, `mediatracknext`, `mediatrackprevious`, `audiovolumeup`, `audiovolumedown`, `audiovolumemute`.
* Actions within a keybind block:
  * `run "command" "args"...` — run external command.
  * `vpn "state_id"` — switch VPN profile.
  * `action "action_name"` — run configured action.
  * `workspace "workspace_name" ["monitor_name"]` — target workspace on a specific monitor.

Example:
```kdl
feature "keybinds" enabled=true {
    bind "super+ctrl+g" {
        action "game-mode"
    }
    bind "alt+b" {
        workspace "browser" "HDMI-0" // focus browser strictly on HDMI-0
    }
}
```

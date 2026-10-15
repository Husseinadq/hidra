First preview of Hidra, a native Linux app for configuring Logitech mice.

## Install (Ubuntu 22.04+, Debian 12+, Linux Mint, Pop!_OS…)

Download `hidra_*_amd64.deb` below, then double-click it, or run:

```bash
sudo apt install ./hidra_*_amd64.deb
```

Open **Hidra** from your app menu. If your receiver was already plugged in,
unplug it and plug it back in once so the new device permission applies.

## What works in this version

- Finds the mouse on a Lightspeed receiver or USB cable, shows name, firmware and battery
- DPI: five stages, the chosen one is sent to the mouse
- Onboard memory on/off and switching between the mouse's 5 saved profiles
- Report rate (only while onboard memory is off; the mouse refuses it otherwise)
- Acceleration, pointer speed and natural scrolling on GNOME

## Not yet

- Button mappings and app profiles are saved in Hidra but **not applied to the mouse yet**
- RGB lighting

Tested on a G502 X PLUS. Reports from other Logitech mice are very welcome:
run `hidra --probe` and paste the output in an issue.

# Linux charge control daemon

A simple daemon to change charge control thresholds for Linux laptops to preserve battery life.

## Usage:
1. Run the daemon as root (for example via systemd service)
2. Write to the pipe:
    
    echo `[SYNTAX]` > /tmp/battery_pipe.


    Syntax examples where `X` is the start threshold and `Y` the end
    - `X to Y`
    - `X .. Y`
    - `start=X`
    - `end=Y`
    - `X..`
    - `..Y`
    - `X to`
    - `to Y`
    - `X`     (if X<50)
    - `Y`     (if 50< Y <= 100)
    - `XtoY`

## Example of cli usage:
```console
# charge_control_daemon --pipe-uid 0 --pipe-gid 0 --pipe-permissions 422 --default-start 20 --default-end 80

$ echo 10 to 90 > /tmp/battery_pipe
```

## Example script using `rofi` and `notify-send`:
```bash
#!/bin/bash

# Paths:
pipe_path=/tmp/battery_pipe
start_file=/sys/class/power_supply/BAT0/charge_control_start_threshold
end_file=/sys/class/power_supply/BAT0/charge_control_end_threshold

# Get value from user
value=$(echo 0% to 100%,15% to 85%,30% to 70%,40% to 60% | rofi -dmenu -sep , -p "Thresholds")

# Write to pipe with timeout
txt=$(timeout -k 0 -s INT 5 bash -c "echo $value > $pipe_path 2>&1" 2>&1)
r=$?

# Get only the error message
err=${txt#*bash: line 1: }

if [ $r == 124 ]; then # Timeout
  notify-send -i /usr/share/icons/breeze-dark/emblems/22/emblem-error.svg "Charge Control" "Timeout while writing to pipe" -u critical
elif [ $r == 1 ]; then # Bash -c error
  notify-send -i /usr/share/icons/breeze-dark/emblems/22/emblem-error.svg "Charge Control" "$err" -u critical
else 
  # Everything is good; Print actual values
  start_th=$(cat $start_file)
  end_th=$(cat $end_file)

  notify-send "Charge Control" "Set thresholds: $start_th% to $end_th%"
fi
```

## Example systemd service
```ini
[Unit]
Description=Battery charge threshold setter

StartLimitIntervalSec=500
StartLimitBurst=5

[Service]
Restart=on-failure
RestartSec=5s
Type=simple
User=root
ExecStart=<PATH TO>/charge_control_daemon --pipe-uid 0 --pipe-gid 0 --pipe-permissions 422 --default-start 20 --default-end 80

[Install]
WantedBy=multi-user.target
```
